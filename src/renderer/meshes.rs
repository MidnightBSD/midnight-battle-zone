use glam::Vec3;
use std::collections::HashMap;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex3D {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex2D {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct CpuMesh {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct MeshRange {
    pub index_start: u32,
    pub index_count: u32,
}

pub struct MeshLibrary {
    meshes: HashMap<String, CpuMesh>,
    pub combined_vertices: Vec<Vertex3D>,
    pub combined_indices: Vec<u32>,
    mesh_ranges: HashMap<String, MeshRange>,
}

impl MeshLibrary {
    pub fn new() -> Self {
        let mut meshes = HashMap::new();

        meshes.insert("terrain".to_string(), build_terrain_mesh());
        meshes.insert("obstacle".to_string(), build_obstacle_mesh());
        meshes.insert("resource_node".to_string(), build_resource_node_mesh());
        meshes.insert("command_center".to_string(), build_command_center_mesh());
        meshes.insert("house".to_string(), build_house_mesh());
        meshes.insert("mine".to_string(), build_mine_mesh());
        meshes.insert("barracks".to_string(), build_barracks_mesh());
        meshes.insert("turret".to_string(), build_turret_mesh());

        // Cat units!
        meshes.insert("worker".to_string(), build_worker_cat_mesh());
        meshes.insert("soldier".to_string(), build_soldier_cat_mesh());
        meshes.insert("sniper".to_string(), build_sniper_cat_mesh());
        meshes.insert("heavy".to_string(), build_heavy_cat_mesh());

        meshes.insert("projectile".to_string(), build_projectile_mesh());
        meshes.insert("selection_ring".to_string(), build_selection_ring_mesh());

        let mut combined_vertices = Vec::new();
        let mut combined_indices = Vec::new();
        let mut mesh_ranges = HashMap::new();

        for (name, mesh) in &meshes {
            let base_vertex = combined_vertices.len() as u32;
            let index_start = combined_indices.len() as u32;
            let index_count = mesh.indices.len() as u32;

            combined_vertices.extend_from_slice(&mesh.vertices);
            for &idx in &mesh.indices {
                combined_indices.push(base_vertex + idx);
            }

            mesh_ranges.insert(
                name.clone(),
                MeshRange {
                    index_start,
                    index_count,
                },
            );
        }

        Self {
            meshes,
            combined_vertices,
            combined_indices,
            mesh_ranges,
        }
    }

    pub fn get_range(&self, name: &str) -> Option<MeshRange> {
        self.mesh_ranges.get(name).copied()
    }

    pub fn get_mesh(&self, name: &str) -> Option<&CpuMesh> {
        self.meshes.get(name)
    }

    pub fn archetypes(&self) -> impl Iterator<Item = &String> {
        self.meshes.keys()
    }
}

// ---------------------------------------------------------------------------
// 3D Procedural Mesh Helpers
// ---------------------------------------------------------------------------

fn add_quad(
    verts: &mut Vec<Vertex3D>,
    inds: &mut Vec<u32>,
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    normal: Vec3,
    color: [f32; 3],
) {
    let start = verts.len() as u32;
    let n = normal.to_array();
    verts.push(Vertex3D { pos: p0.to_array(), normal: n, color });
    verts.push(Vertex3D { pos: p1.to_array(), normal: n, color });
    verts.push(Vertex3D { pos: p2.to_array(), normal: n, color });
    verts.push(Vertex3D { pos: p3.to_array(), normal: n, color });

    inds.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
}

fn add_box(
    verts: &mut Vec<Vertex3D>,
    inds: &mut Vec<u32>,
    center: Vec3,
    size: Vec3,
    color: [f32; 3],
) {
    let h = size * 0.5;
    let min = center - h;
    let max = center + h;

    // +Y (Top)
    add_quad(
        verts, inds,
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::Y,
        color,
    );

    // -Y (Bottom)
    let bot_col = [color[0] * 0.6, color[1] * 0.6, color[2] * 0.6];
    add_quad(
        verts, inds,
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::NEG_Y,
        bot_col,
    );

    // +Z (Front)
    let front_col = [color[0] * 0.9, color[1] * 0.9, color[2] * 0.9];
    add_quad(
        verts, inds,
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::Z,
        front_col,
    );

    // -Z (Back)
    let back_col = [color[0] * 0.75, color[1] * 0.75, color[2] * 0.75];
    add_quad(
        verts, inds,
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::NEG_Z,
        back_col,
    );

    // -X (Left)
    let left_col = [color[0] * 0.8, color[1] * 0.8, color[2] * 0.8];
    add_quad(
        verts, inds,
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::NEG_X,
        left_col,
    );

    // +X (Right)
    let right_col = [color[0] * 0.85, color[1] * 0.85, color[2] * 0.85];
    add_quad(
        verts, inds,
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::X,
        right_col,
    );
}

fn add_prism(
    verts: &mut Vec<Vertex3D>,
    inds: &mut Vec<u32>,
    center: Vec3,
    radius: f32,
    height: f32,
    sides: u32,
    color: [f32; 3],
) {
    let y_bot = center.y;
    let y_top = center.y + height;
    let step = std::f32::consts::TAU / (sides as f32);

    for i in 0..sides {
        let a0 = (i as f32) * step;
        let a1 = ((i + 1) as f32) * step;

        let x0 = a0.sin() * radius + center.x;
        let z0 = a0.cos() * radius + center.z;
        let x1 = a1.sin() * radius + center.x;
        let z1 = a1.cos() * radius + center.z;

        let mid_a = (a0 + a1) * 0.5;
        let normal = Vec3::new(mid_a.sin(), 0.0, mid_a.cos());

        let p0 = Vec3::new(x0, y_bot, z0);
        let p1 = Vec3::new(x1, y_bot, z1);
        let p2 = Vec3::new(x1, y_top, z1);
        let p3 = Vec3::new(x0, y_top, z0);

        let shade = 0.75 + (i % 3) as f32 * 0.1;
        let side_col = [color[0] * shade, color[1] * shade, color[2] * shade];

        add_quad(verts, inds, p0, p1, p2, p3, normal, side_col);
    }

    // Top cap
    let top_center = Vec3::new(center.x, y_top, center.z);
    for i in 0..sides {
        let a0 = (i as f32) * step;
        let a1 = ((i + 1) as f32) * step;
        let p0 = Vec3::new(a0.sin() * radius + center.x, y_top, a0.cos() * radius + center.z);
        let p1 = Vec3::new(a1.sin() * radius + center.x, y_top, a1.cos() * radius + center.z);

        let start = verts.len() as u32;
        let n = Vec3::Y.to_array();
        verts.push(Vertex3D { pos: top_center.to_array(), normal: n, color });
        verts.push(Vertex3D { pos: p0.to_array(), normal: n, color });
        verts.push(Vertex3D { pos: p1.to_array(), normal: n, color });
        inds.extend_from_slice(&[start, start + 1, start + 2]);
    }
}

fn add_pyramid(
    verts: &mut Vec<Vertex3D>,
    inds: &mut Vec<u32>,
    apex: Vec3,
    base_center: Vec3,
    half_w: f32,
    half_d: f32,
    color: [f32; 3],
) {
    let p0 = base_center + Vec3::new(-half_w, 0.0, -half_d);
    let p1 = base_center + Vec3::new(half_w, 0.0, -half_d);
    let p2 = base_center + Vec3::new(half_w, 0.0, half_d);
    let p3 = base_center + Vec3::new(-half_w, 0.0, half_d);

    let faces = [(p0, p1), (p1, p2), (p2, p3), (p3, p0)];
    for (i, (b0, b1)) in faces.iter().enumerate() {
        let edge = *b1 - *b0;
        let to_apex = apex - *b0;
        let n = edge.cross(to_apex).normalize().to_array();
        let shade = 0.8 + (i as f32) * 0.08;
        let col = [color[0] * shade, color[1] * shade, color[2] * shade];

        let start = verts.len() as u32;
        verts.push(Vertex3D { pos: apex.to_array(), normal: n, color: col });
        verts.push(Vertex3D { pos: b0.to_array(), normal: n, color: col });
        verts.push(Vertex3D { pos: b1.to_array(), normal: n, color: col });
        inds.extend_from_slice(&[start, start + 1, start + 2]);
    }
}

fn add_ring(
    verts: &mut Vec<Vertex3D>,
    inds: &mut Vec<u32>,
    center: Vec3,
    r_in: f32,
    r_out: f32,
    segs: u32,
    color: [f32; 3],
) {
    let step = std::f32::consts::TAU / (segs as f32);
    for i in 0..segs {
        let a0 = (i as f32) * step;
        let a1 = ((i + 1) as f32) * step;

        let in0 = center + Vec3::new(a0.sin() * r_in, 0.0, a0.cos() * r_in);
        let out0 = center + Vec3::new(a0.sin() * r_out, 0.0, a0.cos() * r_out);
        let in1 = center + Vec3::new(a1.sin() * r_in, 0.0, a1.cos() * r_in);
        let out1 = center + Vec3::new(a1.sin() * r_out, 0.0, a1.cos() * r_out);

        add_quad(verts, inds, in0, out0, out1, in1, Vec3::Y, color);
    }
}

// ---------------------------------------------------------------------------
// Archetype Mesh Generators
// ---------------------------------------------------------------------------

fn build_terrain_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    // 16x16 tiled tactical terrain grid over (0..1, 0..1) for clear spatial perception
    let grid_div = 16;
    let step = 1.0 / (grid_div as f32);
    let tile_light = [0.34, 0.46, 0.36];
    let tile_alt = [0.30, 0.42, 0.32];

    for gz in 0..grid_div {
        let z0 = (gz as f32) * step;
        let z1 = z0 + step;
        for gx in 0..grid_div {
            let x0 = (gx as f32) * step;
            let x1 = x0 + step;
            let color = if (gx + gz) % 2 == 0 {
                tile_light
            } else {
                tile_alt
            };
            add_quad(
                &mut verts,
                &mut inds,
                Vec3::new(x0, 0.0, z0),
                Vec3::new(x0, 0.0, z1),
                Vec3::new(x1, 0.0, z1),
                Vec3::new(x1, 0.0, z0),
                Vec3::Y,
                color,
            );
        }
    }

    CpuMesh { vertices: verts, indices: inds }
}

fn build_obstacle_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    let rock_col = [0.32, 0.30, 0.28];
    let dark_rock = [0.24, 0.22, 0.20];

    // Clustered jagged rock boulders
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 0.0, 0.0), 1.1, 1.8, 6, rock_col);
    add_pyramid(&mut verts, &mut inds, Vec3::new(0.1, 2.4, -0.1), Vec3::new(0.0, 1.8, 0.0), 0.9, 0.9, rock_col);

    add_prism(&mut verts, &mut inds, Vec3::new(0.8, 0.0, 0.3), 0.7, 1.2, 5, dark_rock);
    add_prism(&mut verts, &mut inds, Vec3::new(-0.7, 0.0, -0.4), 0.8, 1.4, 7, dark_rock);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_resource_node_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    // Dark ore base mound
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 0.0, 0.0), 1.2, 0.4, 8, [0.22, 0.22, 0.25]);

    // 3 Angled glowing crystal prisms
    let crystal_cyan = [0.15, 0.88, 0.98];
    let crystal_bright = [0.35, 0.95, 1.0];

    // Center tall crystal
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 0.3, 0.0), 0.35, 1.6, 6, crystal_cyan);
    add_pyramid(&mut verts, &mut inds, Vec3::new(0.0, 2.3, 0.0), Vec3::new(0.0, 1.9, 0.0), 0.35, 0.35, crystal_bright);

    // Left tilted crystal
    add_prism(&mut verts, &mut inds, Vec3::new(-0.45, 0.3, 0.25), 0.28, 1.2, 6, crystal_cyan);
    add_pyramid(&mut verts, &mut inds, Vec3::new(-0.6, 1.7, 0.35), Vec3::new(-0.45, 1.5, 0.25), 0.28, 0.28, crystal_bright);

    // Right tilted crystal
    add_prism(&mut verts, &mut inds, Vec3::new(0.45, 0.3, -0.2), 0.3, 1.3, 6, crystal_cyan);
    add_pyramid(&mut verts, &mut inds, Vec3::new(0.65, 1.85, -0.3), Vec3::new(0.45, 1.6, -0.2), 0.3, 0.3, crystal_bright);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_command_center_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    let base_steel = [0.34, 0.37, 0.42];
    let tower_steel = [0.26, 0.29, 0.33];
    let accent_gold = [0.92, 0.72, 0.15];

    // Octagonal HQ base
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 0.0, 0.0), 2.2, 0.9, 8, base_steel);

    // Command deck mid tier
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 0.9, 0.0), 1.5, 0.7, 8, tower_steel);

    // Central command tower
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 1.6, 0.0), 0.8, 1.1, 6, base_steel);

    // Radar dish dome
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 2.7, 0.0), 0.5, 0.3, 8, accent_gold);

    // Angled radar dish
    add_box(&mut verts, &mut inds, Vec3::new(0.2, 3.2, 0.2), Vec3::new(0.6, 0.08, 0.6), accent_gold);
    add_box(&mut verts, &mut inds, Vec3::new(0.2, 3.0, 0.2), Vec3::new(0.08, 0.4, 0.08), [0.8, 0.8, 0.8]);

    // Entrance blast door bay
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.4, 1.9), Vec3::new(0.9, 0.6, 0.4), [0.15, 0.15, 0.18]);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_house_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    let wall_col = [0.42, 0.45, 0.48];
    let roof_col = [0.75, 0.38, 0.18];
    let trim_col = [0.95, 0.7, 0.1];

    // Main habitat block
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.8, 1.0, 1.4), wall_col);

    // Sloped roof (pyramid top)
    add_pyramid(&mut verts, &mut inds, Vec3::new(0.0, 1.6, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.95, 0.75, roof_col);

    // Side annex / solar generator module
    add_box(&mut verts, &mut inds, Vec3::new(1.1, 0.35, 0.1), Vec3::new(0.6, 0.7, 0.9), wall_col);
    add_box(&mut verts, &mut inds, Vec3::new(1.1, 0.72, 0.1), Vec3::new(0.55, 0.06, 0.85), trim_col);

    // Roof antenna
    add_box(&mut verts, &mut inds, Vec3::new(-0.5, 1.8, 0.2), Vec3::new(0.05, 0.6, 0.05), [0.8, 0.8, 0.8]);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_mine_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    let frame_col = [0.30, 0.31, 0.34];
    let amber_col = [0.88, 0.55, 0.12];

    // Heavy concrete foundation
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.2, 0.0), Vec3::new(2.0, 0.4, 2.0), frame_col);

    // 4 Corner structural pillars
    for &x in &[-0.75, 0.75] {
        for &z in &[-0.75, 0.75] {
            add_box(&mut verts, &mut inds, Vec3::new(x, 1.3, z), Vec3::new(0.2, 2.0, 0.2), frame_col);
        }
    }

    // Top drill platform
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 2.3, 0.0), Vec3::new(1.8, 0.3, 1.8), amber_col);

    // Central drill shaft
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 0.0, 0.0), 0.45, 2.4, 8, [0.18, 0.18, 0.2]);

    // Side ore hopper / collection bin
    add_box(&mut verts, &mut inds, Vec3::new(1.1, 0.6, 0.0), Vec3::new(0.7, 0.8, 1.1), amber_col);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_barracks_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    let bunker_green = [0.28, 0.34, 0.25];
    let steel_grey = [0.38, 0.40, 0.42];
    let warning_stripe = [0.85, 0.65, 0.15];

    // Main fortified military bunker
    add_box(&mut verts, &mut inds, Vec3::new(-0.3, 0.6, 0.0), Vec3::new(1.8, 1.2, 2.0), bunker_green);

    // Twin roof exhaust ventilation cylinders
    add_prism(&mut verts, &mut inds, Vec3::new(-0.7, 1.2, 0.5), 0.25, 0.5, 6, steel_grey);
    add_prism(&mut verts, &mut inds, Vec3::new(-0.7, 1.2, -0.5), 0.25, 0.5, 6, steel_grey);

    // Armory annex / training courtyard with barrier walls
    add_box(&mut verts, &mut inds, Vec3::new(0.9, 0.2, 0.0), Vec3::new(0.9, 0.4, 1.8), steel_grey);
    add_box(&mut verts, &mut inds, Vec3::new(1.3, 0.5, 0.0), Vec3::new(0.12, 0.4, 1.8), warning_stripe);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_turret_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    let base_armor = [0.28, 0.31, 0.36];
    let gun_slate = [0.20, 0.22, 0.26];
    let barrel_steel = [0.46, 0.50, 0.56];
    let sensor_cyan = [0.15, 0.90, 1.0];
    let hazard_gold = [0.92, 0.72, 0.16];

    // Heavy octagonal armored foundation & turret ring
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 0.0, 0.0), 1.45, 0.55, 8, base_armor);
    add_prism(&mut verts, &mut inds, Vec3::new(0.0, 0.55, 0.0), 1.05, 0.45, 8, gun_slate);
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.56, 0.0), Vec3::new(2.2, 0.12, 0.45), hazard_gold);

    // Rotating gunnery cupola (faces +X so rotation_yaw = atan2(dy, dx) aims barrels at target)
    add_box(&mut verts, &mut inds, Vec3::new(-0.05, 1.35, 0.0), Vec3::new(1.15, 0.72, 1.05), base_armor);

    // Pointy Cat-Ear Radar / Sensor Fins atop the turret cupola
    add_pyramid(&mut verts, &mut inds, Vec3::new(-0.20, 2.15, -0.34), Vec3::new(-0.20, 1.71, -0.34), 0.16, 0.16, gun_slate);
    add_pyramid(&mut verts, &mut inds, Vec3::new(-0.20, 2.15, 0.34), Vec3::new(-0.20, 1.71, 0.34), 0.16, 0.16, gun_slate);

    // Glowing optical targeting visor on front (+X face)
    add_box(&mut verts, &mut inds, Vec3::new(0.54, 1.48, 0.0), Vec3::new(0.08, 0.18, 0.56), sensor_cyan);

    // Twin Heavy Autocannon Barrels extending along +X
    for &z_off in &[-0.26, 0.26] {
        // Breech housing
        add_box(&mut verts, &mut inds, Vec3::new(0.65, 1.30, z_off), Vec3::new(0.55, 0.24, 0.22), gun_slate);
        // Long autocannon barrel
        add_box(&mut verts, &mut inds, Vec3::new(1.35, 1.30, z_off), Vec3::new(1.25, 0.13, 0.13), barrel_steel);
        // Muzzle brake
        add_box(&mut verts, &mut inds, Vec3::new(1.98, 1.30, z_off), Vec3::new(0.18, 0.19, 0.19), hazard_gold);
    }

    CpuMesh { vertices: verts, indices: inds }
}

// ---------------------------------------------------------------------------
// 3D CAT Meshes (Worker Cat, Soldier Cat, Sniper Cat)
// ---------------------------------------------------------------------------

fn build_cat_base(
    verts: &mut Vec<Vertex3D>,
    inds: &mut Vec<u32>,
    fur_color: [f32; 3],
    belly_color: [f32; 3],
) {
    // 1. Feline Body / Torso
    add_box(verts, inds, Vec3::new(0.0, 0.45, 0.0), Vec3::new(0.48, 0.38, 0.78), fur_color);
    // Underbelly patch
    add_box(verts, inds, Vec3::new(0.0, 0.32, 0.0), Vec3::new(0.36, 0.10, 0.65), belly_color);

    // 2. Cat Head
    add_box(verts, inds, Vec3::new(0.0, 0.66, 0.44), Vec3::new(0.44, 0.38, 0.38), fur_color);

    // 3. Muzzle / Snout
    add_box(verts, inds, Vec3::new(0.0, 0.58, 0.64), Vec3::new(0.22, 0.16, 0.14), belly_color);
    // Pink nose
    add_box(verts, inds, Vec3::new(0.0, 0.63, 0.71), Vec3::new(0.06, 0.04, 0.03), [0.98, 0.6, 0.7]);

    // 4. Pointy Triangular Cat Ears (with soft pink inside)
    let ear_pink = [0.95, 0.75, 0.8];
    // Left ear
    add_pyramid(verts, inds, Vec3::new(-0.14, 1.02, 0.42), Vec3::new(-0.14, 0.85, 0.42), 0.08, 0.08, fur_color);
    add_box(verts, inds, Vec3::new(-0.13, 0.90, 0.46), Vec3::new(0.07, 0.12, 0.02), ear_pink);
    // Right ear
    add_pyramid(verts, inds, Vec3::new(0.14, 1.02, 0.42), Vec3::new(0.14, 0.85, 0.42), 0.08, 0.08, fur_color);
    add_box(verts, inds, Vec3::new(0.13, 0.90, 0.46), Vec3::new(0.07, 0.12, 0.02), ear_pink);

    // 5. Glowing Feline Eyes
    let eye_green = [0.2, 0.95, 0.35];
    add_box(verts, inds, Vec3::new(-0.12, 0.70, 0.63), Vec3::new(0.07, 0.05, 0.03), eye_green);
    add_box(verts, inds, Vec3::new(0.12, 0.70, 0.63), Vec3::new(0.07, 0.05, 0.03), eye_green);

    // 6. Four Cat Legs and Paws
    for &x in &[-0.18, 0.18] {
        // Front paws
        add_box(verts, inds, Vec3::new(x, 0.20, 0.28), Vec3::new(0.13, 0.38, 0.13), fur_color);
        add_box(verts, inds, Vec3::new(x, 0.05, 0.31), Vec3::new(0.15, 0.10, 0.16), belly_color);

        // Rear paws
        add_box(verts, inds, Vec3::new(x, 0.20, -0.28), Vec3::new(0.14, 0.38, 0.15), fur_color);
        add_box(verts, inds, Vec3::new(x, 0.05, -0.26), Vec3::new(0.15, 0.10, 0.16), belly_color);
    }

    // 7. Expressive Angled Cat Tail (curves up and back)
    add_box(verts, inds, Vec3::new(0.0, 0.58, -0.45), Vec3::new(0.09, 0.18, 0.16), fur_color);
    add_box(verts, inds, Vec3::new(0.04, 0.78, -0.56), Vec3::new(0.08, 0.24, 0.10), fur_color);
    // Tail tip (white tip accent)
    add_box(verts, inds, Vec3::new(0.07, 0.96, -0.52), Vec3::new(0.09, 0.16, 0.09), belly_color);
}

fn build_worker_cat_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    // Calico / Orange Tabby Worker Cat
    let tabby_orange = [0.90, 0.52, 0.20];
    let white_fur = [0.95, 0.94, 0.90];
    build_cat_base(&mut verts, &mut inds, tabby_orange, white_fur);

    // Yellow Safety Hard-Hat between pointy cat ears!
    let hardhat_yellow = [0.98, 0.82, 0.06];
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.86, 0.44), Vec3::new(0.30, 0.10, 0.32), hardhat_yellow);
    // Hardhat rim
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.82, 0.46), Vec3::new(0.34, 0.03, 0.36), hardhat_yellow);

    // Utility Backpack on cat back
    let pack_slate = [0.35, 0.38, 0.42];
    let warning_orange = [0.95, 0.45, 0.08];
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.68, -0.04), Vec3::new(0.36, 0.26, 0.34), pack_slate);
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.70, -0.22), Vec3::new(0.28, 0.18, 0.08), warning_orange);

    // Robotic harvesting / welding arm on right side
    let arm_metal = [0.25, 0.26, 0.28];
    add_box(&mut verts, &mut inds, Vec3::new(0.30, 0.48, 0.10), Vec3::new(0.12, 0.12, 0.35), arm_metal);
    add_box(&mut verts, &mut inds, Vec3::new(0.32, 0.42, 0.40), Vec3::new(0.10, 0.22, 0.10), arm_metal);
    // Cyan welding tip / claw
    add_box(&mut verts, &mut inds, Vec3::new(0.32, 0.32, 0.55), Vec3::new(0.08, 0.08, 0.20), [0.2, 0.9, 1.0]);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_soldier_cat_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    // Tactical Tuxedo / Slate Grey Soldier Cat
    let slate_fur = [0.24, 0.26, 0.30];
    let white_fur = [0.92, 0.92, 0.92];
    build_cat_base(&mut verts, &mut inds, slate_fur, white_fur);

    // Tactical Flak Vest / Combat Harness
    let vest_camo = [0.22, 0.32, 0.24];
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.46, 0.02), Vec3::new(0.52, 0.42, 0.64), vest_camo);

    // Shoulder pauldrons / armor pads
    let pad_metal = [0.38, 0.42, 0.46];
    add_box(&mut verts, &mut inds, Vec3::new(-0.27, 0.58, 0.22), Vec3::new(0.12, 0.14, 0.22), pad_metal);
    add_box(&mut verts, &mut inds, Vec3::new(0.27, 0.58, 0.22), Vec3::new(0.12, 0.14, 0.22), pad_metal);

    // Tactical Assault Rifle
    let gun_black = [0.12, 0.13, 0.15];
    // Main body
    add_box(&mut verts, &mut inds, Vec3::new(0.30, 0.42, 0.35), Vec3::new(0.10, 0.14, 0.45), gun_black);
    // Long barrel extending forward
    add_box(&mut verts, &mut inds, Vec3::new(0.30, 0.44, 0.68), Vec3::new(0.05, 0.05, 0.32), [0.4, 0.4, 0.45]);
    // Curved banana magazine
    add_box(&mut verts, &mut inds, Vec3::new(0.30, 0.28, 0.38), Vec3::new(0.06, 0.16, 0.12), gun_black);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_sniper_cat_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    // Midnight Black Shadow Sniper Cat
    let shadow_fur = [0.11, 0.12, 0.14];
    let dark_grey = [0.30, 0.32, 0.36];
    build_cat_base(&mut verts, &mut inds, shadow_fur, dark_grey);

    // Sleek Recon Cloak / Mantle over shoulders and back
    let cloak_blue = [0.15, 0.20, 0.28];
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.48, -0.02), Vec3::new(0.52, 0.42, 0.72), cloak_blue);

    // Cybernetic Recon Eye Visor over one eye
    let visor_cyan = [0.0, 0.95, 1.0];
    add_box(&mut verts, &mut inds, Vec3::new(0.12, 0.70, 0.65), Vec3::new(0.10, 0.08, 0.06), visor_cyan);

    // Heavy High-Precision Sniper Rifle (long barrel with scope)
    let rifle_metal = [0.14, 0.15, 0.17];
    // Rifle body
    add_box(&mut verts, &mut inds, Vec3::new(0.28, 0.40, 0.30), Vec3::new(0.09, 0.12, 0.55), rifle_metal);
    // Extra long sniper barrel with muzzle brake
    add_box(&mut verts, &mut inds, Vec3::new(0.28, 0.42, 0.85), Vec3::new(0.04, 0.04, 0.65), [0.5, 0.52, 0.56]);
    add_box(&mut verts, &mut inds, Vec3::new(0.28, 0.42, 1.20), Vec3::new(0.08, 0.08, 0.08), [0.3, 0.3, 0.3]);
    // High-power optical scope on top
    add_box(&mut verts, &mut inds, Vec3::new(0.28, 0.53, 0.32), Vec3::new(0.06, 0.08, 0.32), [0.08, 0.09, 0.10]);
    // Glowing red scope lens
    add_box(&mut verts, &mut inds, Vec3::new(0.28, 0.53, 0.49), Vec3::new(0.05, 0.07, 0.02), [1.0, 0.15, 0.15]);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_heavy_cat_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    // Heavy Armored Charcoal / White Tuxedo Cat Specialist
    let heavy_fur = [0.18, 0.20, 0.24];
    let white_bib = [0.92, 0.93, 0.95];
    build_cat_base(&mut verts, &mut inds, heavy_fur, white_bib);

    // Heavy Juggernaut Blast Armor & Chest Plate (2x Soldier Armor)
    let plate_steel = [0.32, 0.36, 0.42];
    let hazard_amber = [0.95, 0.62, 0.12];
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.47, 0.02), Vec3::new(0.58, 0.46, 0.72), plate_steel);
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.52, 0.39), Vec3::new(0.42, 0.16, 0.04), hazard_amber);

    // Oversized Heavy Shoulder Pauldrons
    add_box(&mut verts, &mut inds, Vec3::new(-0.31, 0.60, 0.18), Vec3::new(0.16, 0.18, 0.28), plate_steel);
    add_box(&mut verts, &mut inds, Vec3::new(0.31, 0.60, 0.18), Vec3::new(0.16, 0.18, 0.28), plate_steel);

    // Twin Rocket Ammo Pack on Back
    let tube_olive = [0.22, 0.30, 0.20];
    let warhead_red = [0.92, 0.20, 0.18];
    add_box(&mut verts, &mut inds, Vec3::new(-0.10, 0.72, -0.18), Vec3::new(0.14, 0.36, 0.14), tube_olive);
    add_box(&mut verts, &mut inds, Vec3::new(0.10, 0.72, -0.18), Vec3::new(0.14, 0.36, 0.14), tube_olive);
    add_box(&mut verts, &mut inds, Vec3::new(-0.10, 0.94, -0.18), Vec3::new(0.10, 0.10, 0.10), warhead_red);
    add_box(&mut verts, &mut inds, Vec3::new(0.10, 0.94, -0.18), Vec3::new(0.10, 0.10, 0.10), warhead_red);

    // Shoulder-Mounted Heavy Rocket Launcher (Bazooka)
    let launcher_dark = [0.16, 0.22, 0.16];
    // Main launch tube resting over right shoulder
    add_box(&mut verts, &mut inds, Vec3::new(0.32, 0.68, 0.32), Vec3::new(0.18, 0.18, 1.05), launcher_dark);
    // Flared exhaust vent at rear & reinforced muzzle ring at front
    add_box(&mut verts, &mut inds, Vec3::new(0.32, 0.68, -0.24), Vec3::new(0.22, 0.22, 0.12), [0.28, 0.28, 0.30]);
    add_box(&mut verts, &mut inds, Vec3::new(0.32, 0.68, 0.86), Vec3::new(0.22, 0.22, 0.10), hazard_amber);
    // Loaded red-tipped rocket warhead protruding from front of launcher
    add_box(&mut verts, &mut inds, Vec3::new(0.32, 0.68, 0.98), Vec3::new(0.12, 0.12, 0.18), warhead_red);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_projectile_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    let plasma_bright = [1.0, 0.95, 0.4];
    add_box(&mut verts, &mut inds, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.08, 0.08, 0.8), plasma_bright);

    CpuMesh { vertices: verts, indices: inds }
}

fn build_selection_ring_mesh() -> CpuMesh {
    let mut verts = Vec::new();
    let mut inds = Vec::new();

    let ring_green = [0.2, 1.0, 0.3];
    add_ring(&mut verts, &mut inds, Vec3::new(0.0, 0.02, 0.0), 0.85, 1.05, 28, ring_green);

    CpuMesh { vertices: verts, indices: inds }
}

// ---------------------------------------------------------------------------
// 2D Geometry & 5x7 ASCII Bitmap Font Renderer
// ---------------------------------------------------------------------------

pub fn append_rect(vertices: &mut Vec<Vertex2D>, rect: [f32; 4], color: [f32; 4]) {
    let [x, y, w, h] = rect;
    let v0 = Vertex2D { pos: [x, y], uv: [0.0, 0.0], color };
    let v1 = Vertex2D { pos: [x + w, y], uv: [1.0, 0.0], color };
    let v2 = Vertex2D { pos: [x + w, y + h], uv: [1.0, 1.0], color };
    let v3 = Vertex2D { pos: [x, y + h], uv: [0.0, 1.0], color };

    vertices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
}

pub fn append_triangle(
    vertices: &mut Vec<Vertex2D>,
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    color: [f32; 4],
) {
    let v0 = Vertex2D { pos: p0, uv: [0.0, 0.0], color };
    let v1 = Vertex2D { pos: p1, uv: [0.5, 0.0], color };
    let v2 = Vertex2D { pos: p2, uv: [1.0, 1.0], color };
    vertices.extend_from_slice(&[v0, v1, v2]);
}

pub fn append_rect_outline(
    vertices: &mut Vec<Vertex2D>,
    rect: [f32; 4],
    border: f32,
    color: [f32; 4],
) {
    let [x, y, w, h] = rect;
    // Top
    append_rect(vertices, [x, y, w, border], color);
    // Bottom
    append_rect(vertices, [x, y + h - border, w, border], color);
    // Left
    append_rect(vertices, [x, y + border, border, h - 2.0 * border], color);
    // Right
    append_rect(vertices, [x + w - border, y + border, border, h - 2.0 * border], color);
}

pub fn append_textured_rect(
    vertices: &mut Vec<Vertex2D>,
    rect: [f32; 4],
    uv_rect: [f32; 4],
    color: [f32; 4],
) {
    let [x, y, w, h] = rect;
    let [u0, v0, uw, vh] = uv_rect;
    let u1 = u0 + uw;
    let v1 = v0 + vh;

    let p0 = Vertex2D { pos: [x, y], uv: [u0, v0], color };
    let p1 = Vertex2D { pos: [x + w, y], uv: [u1, v0], color };
    let p2 = Vertex2D { pos: [x + w, y + h], uv: [u1, v1], color };
    let p3 = Vertex2D { pos: [x, y + h], uv: [u0, v1], color };

    vertices.extend_from_slice(&[p0, p1, p2, p0, p2, p3]);
}

/// 5x7 ASCII Bitmap Font Table for characters 32 (' ') to 126 ('~')
fn get_glyph_bitmap(ch: char) -> [u8; 7] {
    let c = ch as u32;
    match c {
        32 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Space
        33 => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04], // !
        34 => [0x0A, 0x0A, 0x0A, 0x00, 0x00, 0x00, 0x00], // "
        35 => [0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A], // #
        36 => [0x04, 0x0F, 0x14, 0x0E, 0x05, 0x1E, 0x04], // $
        37 => [0x19, 0x19, 0x02, 0x04, 0x08, 0x13, 0x13], // %
        38 => [0x08, 0x14, 0x14, 0x08, 0x15, 0x12, 0x0D], // &
        39 => [0x04, 0x04, 0x08, 0x00, 0x00, 0x00, 0x00], // '
        40 => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02], // (
        41 => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08], // )
        42 => [0x00, 0x04, 0x15, 0x0E, 0x15, 0x04, 0x00], // *
        43 => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00], // +
        44 => [0x00, 0x00, 0x00, 0x00, 0x04, 0x04, 0x08], // ,
        45 => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00], // -
        46 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04], // .
        47 => [0x01, 0x02, 0x04, 0x08, 0x10, 0x00, 0x00], // /

        // Digits 0-9
        48 => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E], // 0
        49 => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E], // 1
        50 => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F], // 2
        51 => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E], // 3
        52 => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02], // 4
        53 => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E], // 5
        54 => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E], // 6
        55 => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08], // 7
        56 => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E], // 8
        57 => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C], // 9

        58 => [0x00, 0x04, 0x00, 0x00, 0x04, 0x00, 0x00], // :
        59 => [0x00, 0x04, 0x00, 0x00, 0x04, 0x04, 0x08], // ;
        60 => [0x02, 0x04, 0x08, 0x10, 0x08, 0x04, 0x02], // <
        61 => [0x00, 0x1F, 0x00, 0x1F, 0x00, 0x00, 0x00], // =
        62 => [0x08, 0x04, 0x02, 0x01, 0x02, 0x04, 0x08], // >
        63 => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04], // ?
        64 => [0x0E, 0x11, 0x17, 0x15, 0x17, 0x10, 0x0F], // @

        // Uppercase A-Z
        65 => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11], // A
        66 => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E], // B
        67 => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E], // C
        68 => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C], // D
        69 => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F], // E
        70 => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10], // F
        71 => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F], // G
        72 => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11], // H
        73 => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E], // I
        74 => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C], // J
        75 => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11], // K
        76 => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F], // L
        77 => [0x11, 0x1B, 0x15, 0x11, 0x11, 0x11, 0x11], // M
        78 => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11], // N
        79 => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E], // O
        80 => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10], // P
        81 => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D], // Q
        82 => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11], // R
        83 => [0x0E, 0x11, 0x10, 0x0E, 0x01, 0x11, 0x0E], // S
        84 => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04], // T
        85 => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E], // U
        86 => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04], // V
        87 => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A], // W
        88 => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11], // X
        89 => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04], // Y
        90 => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F], // Z

        91 => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E], // [
        92 => [0x10, 0x08, 0x04, 0x02, 0x01, 0x00, 0x00], // \
        93 => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E], // ]
        94 => [0x04, 0x0A, 0x11, 0x00, 0x00, 0x00, 0x00], // ^
        95 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F], // _

        // Lowercase a-z
        97 => [0x00, 0x00, 0x0E, 0x01, 0x0F, 0x11, 0x0F],  // a
        98 => [0x10, 0x10, 0x16, 0x19, 0x11, 0x11, 0x1E],  // b
        99 => [0x00, 0x00, 0x0E, 0x10, 0x10, 0x11, 0x0E],  // c
        100 => [0x01, 0x01, 0x0D, 0x13, 0x11, 0x11, 0x0F], // d
        101 => [0x00, 0x00, 0x0E, 0x11, 0x1F, 0x10, 0x0E], // e
        102 => [0x06, 0x09, 0x08, 0x1C, 0x08, 0x08, 0x08], // f
        103 => [0x00, 0x00, 0x0F, 0x11, 0x0F, 0x01, 0x0E], // g
        104 => [0x10, 0x10, 0x16, 0x19, 0x11, 0x11, 0x11], // h
        105 => [0x04, 0x00, 0x0C, 0x04, 0x04, 0x04, 0x0E], // i
        106 => [0x02, 0x00, 0x06, 0x02, 0x02, 0x12, 0x0C], // j
        107 => [0x10, 0x10, 0x12, 0x14, 0x18, 0x14, 0x12], // k
        108 => [0x0C, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E], // l
        109 => [0x00, 0x00, 0x1A, 0x15, 0x15, 0x11, 0x11], // m
        110 => [0x00, 0x00, 0x16, 0x19, 0x11, 0x11, 0x11], // n
        111 => [0x00, 0x00, 0x0E, 0x11, 0x11, 0x11, 0x0E], // o
        112 => [0x00, 0x00, 0x1E, 0x11, 0x1E, 0x10, 0x10], // p
        113 => [0x00, 0x00, 0x0D, 0x13, 0x0F, 0x01, 0x01], // q
        114 => [0x00, 0x00, 0x16, 0x19, 0x10, 0x10, 0x10], // r
        115 => [0x00, 0x00, 0x0F, 0x10, 0x0E, 0x01, 0x1E], // s
        116 => [0x08, 0x08, 0x1C, 0x08, 0x08, 0x09, 0x06], // t
        117 => [0x00, 0x00, 0x11, 0x11, 0x11, 0x13, 0x0D], // u
        118 => [0x00, 0x00, 0x11, 0x11, 0x11, 0x0A, 0x04], // v
        119 => [0x00, 0x00, 0x11, 0x11, 0x15, 0x15, 0x0A], // w
        120 => [0x00, 0x00, 0x11, 0x0A, 0x04, 0x0A, 0x11], // x
        121 => [0x00, 0x00, 0x11, 0x11, 0x0F, 0x01, 0x0E], // y
        122 => [0x00, 0x00, 0x1F, 0x02, 0x04, 0x08, 0x1F], // z

        123 => [0x02, 0x04, 0x04, 0x08, 0x04, 0x04, 0x02], // {
        124 => [0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04], // |
        125 => [0x08, 0x04, 0x04, 0x02, 0x04, 0x04, 0x08], // }
        126 => [0x00, 0x08, 0x15, 0x02, 0x00, 0x00, 0x00], // ~

        _ => [0x1F, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1F], // Default box
    }
}

pub fn append_text_quads(
    vertices: &mut Vec<Vertex2D>,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: [f32; 4],
) -> f32 {
    let mut cur_x = x;
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        let glyph = get_glyph_bitmap(ch);
        for row in 0..7 {
            let row_bits = glyph[row];
            for col in 0..5 {
                if (row_bits & (1 << (4 - col))) != 0 {
                    let px = cur_x + (col as f32) * scale;
                    let py = y + (row as f32) * scale;
                    append_rect(vertices, [px, py, scale, scale], color);
                }
            }
        }
        cur_x += (5.0 + 1.0) * scale;
    }
    cur_x - x
}
