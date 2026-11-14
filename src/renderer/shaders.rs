use naga::back::spv;
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};

pub const WORLD_SHADER_WGSL: &str = r#"
struct PushConstants {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    tint_color: vec4<f32>,
    map_params: vec4<f32>,
}

var<push_constant> pc: PushConstants;

@group(0) @binding(0)
var fog_texture: texture_2d<f32>;
@group(0) @binding(1)
var fog_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) vertex_color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos4 = pc.model * vec4<f32>(in.position, 1.0);
    out.world_pos = world_pos4.xyz;
    let norm4 = pc.model * vec4<f32>(in.normal, 0.0);
    out.world_normal = normalize(norm4.xyz);
    out.vertex_color = in.color;
    out.clip_position = pc.mvp * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.4));
    let n = normalize(in.world_normal);
    let diff = max(dot(n, light_dir), 0.0);
    let ambient = 0.62;
    let lighting = ambient + diff * 0.48;

    var base_color = in.vertex_color;
    if (pc.tint_color.a > 0.01) {
        base_color = mix(base_color, pc.tint_color.rgb, pc.tint_color.a);
    }

    var fog_visibility = 1.0;
    if (pc.map_params.x > 0.0 && pc.map_params.y > 0.0) {
        let u = clamp(in.world_pos.x / pc.map_params.x, 0.0, 1.0);
        let v = clamp(in.world_pos.z / pc.map_params.y, 0.0, 1.0);
        let fog_sample = textureSample(fog_texture, fog_sampler, vec2<f32>(u, v));
        fog_visibility = fog_sample.r;
    }

    let lit_color = base_color * lighting;
    let final_rgb = lit_color * clamp(fog_visibility, 0.40, 1.0);

    return vec4<f32>(final_rgb, 1.0);
}
"#;

pub const UI_SHADER_WGSL: &str = r#"
struct UiPushConstants {
    screen_size: vec2<f32>,
    use_texture: f32,
    _pad: f32,
}

var<push_constant> pc: UiPushConstants;

@group(0) @binding(0)
var ui_texture: texture_2d<f32>;
@group(0) @binding(1)
var ui_sampler: sampler;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let ndc_x = (2.0 * in.pos.x / pc.screen_size.x) - 1.0;
    let ndc_y = (2.0 * in.pos.y / pc.screen_size.y) - 1.0;
    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (pc.use_texture > 0.5) {
        let tex = textureSample(ui_texture, ui_sampler, in.uv);
        return in.color * tex;
    }
    return in.color;
}
"#;

/// AMD FidelityFX Super Resolution 1.0: Edge-Adaptive Spatial Upsampling (EASU)
pub const FSR_EASU_WGSL: &str = r#"
struct FsrEasuConstants {
    src_size: vec2<f32>,
    dst_size: vec2<f32>,
}

var<push_constant> pc: FsrEasuConstants;

@group(0) @binding(0)
var src_tex: texture_2d<f32>;
@group(0) @binding(1)
var src_smp: sampler;

struct FullscreenVertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    var out: FullscreenVertexOutput;
    let uv = vec2<f32>(
        f32((vertex_index << 1u) & 2u),
        f32(vertex_index & 2u)
    );
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let inv_src = 1.0 / pc.src_size;
    let pos = in.uv * pc.src_size - 0.5;
    let base = floor(pos);
    let frac = pos - base;

    let uv0 = (base + vec2<f32>(0.5, 0.5)) * inv_src;
    let uv_n = uv0 + vec2<f32>(0.0, -inv_src.y);
    let uv_s = uv0 + vec2<f32>(0.0, inv_src.y);
    let uv_w = uv0 + vec2<f32>(-inv_src.x, 0.0);
    let uv_e = uv0 + vec2<f32>(inv_src.x, 0.0);

    let c = textureSample(src_tex, src_smp, uv0).rgb;
    let n = textureSample(src_tex, src_smp, uv_n).rgb;
    let s = textureSample(src_tex, src_smp, uv_s).rgb;
    let w = textureSample(src_tex, src_smp, uv_w).rgb;
    let e = textureSample(src_tex, src_smp, uv_e).rgb;

    let lum_c = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let lum_n = dot(n, vec3<f32>(0.2126, 0.7152, 0.0722));
    let lum_s = dot(s, vec3<f32>(0.2126, 0.7152, 0.0722));
    let lum_w = dot(w, vec3<f32>(0.2126, 0.7152, 0.0722));
    let lum_e = dot(e, vec3<f32>(0.2126, 0.7152, 0.0722));

    let grad_x = abs(lum_e - lum_w);
    let grad_y = abs(lum_s - lum_n);

    let weight_x = clamp(1.0 - grad_x * 0.5, 0.25, 1.0);
    let weight_y = clamp(1.0 - grad_y * 0.5, 0.25, 1.0);

    let color_x = mix(w, e, frac.x);
    let color_y = mix(n, s, frac.y);
    let edge_color = mix(color_x, color_y, weight_y / (weight_x + weight_y));
    let final_color = mix(c, edge_color, 0.75);

    return vec4<f32>(final_color, 1.0);
}
"#;

/// AMD FidelityFX Super Resolution 1.0: Robust Contrast-Adaptive Sharpening (RCAS)
pub const FSR_RCAS_WGSL: &str = r#"
struct FsrRcasConstants {
    dst_size: vec2<f32>,
    sharpness: f32,
    _pad: f32,
}

var<push_constant> pc: FsrRcasConstants;

@group(0) @binding(0)
var src_tex: texture_2d<f32>;
@group(0) @binding(1)
var src_smp: sampler;

struct FullscreenVertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    var out: FullscreenVertexOutput;
    let uv = vec2<f32>(
        f32((vertex_index << 1u) & 2u),
        f32(vertex_index & 2u)
    );
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let step = 1.0 / pc.dst_size;

    let c = textureSample(src_tex, src_smp, in.uv).rgb;
    let n = textureSample(src_tex, src_smp, in.uv + vec2<f32>(0.0, -step.y)).rgb;
    let s = textureSample(src_tex, src_smp, in.uv + vec2<f32>(0.0, step.y)).rgb;
    let w = textureSample(src_tex, src_smp, in.uv + vec2<f32>(-step.x, 0.0)).rgb;
    let e = textureSample(src_tex, src_smp, in.uv + vec2<f32>(step.x, 0.0)).rgb;

    let min_col = min(c, min(min(n, s), min(w, e)));
    let max_col = max(c, max(max(n, s), max(w, e)));

    let lum_min = dot(min_col, vec3<f32>(0.2126, 0.7152, 0.0722));
    let lum_max = dot(max_col, vec3<f32>(0.2126, 0.7152, 0.0722));
    let contrast = lum_max - lum_min;

    let amp = clamp(1.0 - contrast, 0.0, 1.0);
    let weight = -0.18 * pc.sharpness * amp;

    let sharp_rgb = (c + (n + s + w + e) * weight) / (1.0 + 4.0 * weight);
    let clamped_rgb = clamp(sharp_rgb, min_col, max_col);

    return vec4<f32>(clamped_rgb, 1.0);
}
"#;

pub fn compile_wgsl_to_spirv(source: &str, stage_name: &str) -> Result<Vec<u32>, String> {
    let module = wgsl::parse_str(source)
        .map_err(|e| format!("WGSL parse error in {}: {:?}", stage_name, e))?;

    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let info = validator
        .validate(&module)
        .map_err(|e| format!("Validation error in {}: {:?}", stage_name, e))?;

    let flags = spv::WriterFlags::LABEL_VARYINGS;
    let options = spv::Options {
        lang_version: (1, 0),
        flags,
        ..Default::default()
    };

    let spv = spv::write_vec(&module, &info, &options, None)
        .map_err(|e| format!("SPIR-V generation error in {}: {:?}", stage_name, e))?;

    Ok(spv)
}

pub struct ShaderBinaries {
    pub world_spv: Vec<u32>,
    pub ui_spv: Vec<u32>,
    pub fsr_easu_spv: Vec<u32>,
    pub fsr_rcas_spv: Vec<u32>,
}

pub fn compile_all_shaders() -> Result<ShaderBinaries, String> {
    let world_spv = compile_wgsl_to_spirv(WORLD_SHADER_WGSL, "world shader")?;
    let ui_spv = compile_wgsl_to_spirv(UI_SHADER_WGSL, "ui shader")?;
    let fsr_easu_spv = compile_wgsl_to_spirv(FSR_EASU_WGSL, "fsr easu shader")?;
    let fsr_rcas_spv = compile_wgsl_to_spirv(FSR_RCAS_WGSL, "fsr rcas shader")?;

    Ok(ShaderBinaries {
        world_spv,
        ui_spv,
        fsr_easu_spv,
        fsr_rcas_spv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_all_shaders() {
        let res = compile_all_shaders();
        assert!(res.is_ok(), "Shader compilation failed: {:?}", res.err());
        let bins = res.unwrap();
        assert!(!bins.world_spv.is_empty(), "World shader SPIR-V is empty");
        assert!(!bins.ui_spv.is_empty(), "UI shader SPIR-V is empty");
        assert!(!bins.fsr_easu_spv.is_empty(), "FSR EASU SPIR-V is empty");
        assert!(!bins.fsr_rcas_spv.is_empty(), "FSR RCAS SPIR-V is empty");
    }
}
