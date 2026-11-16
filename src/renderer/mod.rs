pub mod camera;
pub mod meshes;
pub mod shaders;

pub use camera::IsometricCamera;
pub use meshes::{CpuMesh, MeshLibrary, MeshRange, Vertex2D, Vertex3D};

use ash::vk;
use glam::{Mat4, Vec2, Vec3};
use std::ffi::CStr;

use crate::types::RenderInstance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsrMode {
    Off,
    UltraQuality,
    Quality,
    Balanced,
    Performance,
}

impl FsrMode {
    pub fn scale_factor(self) -> f32 {
        match self {
            FsrMode::Off => 1.0,
            FsrMode::UltraQuality => 1.3,
            FsrMode::Quality => 1.5,
            FsrMode::Balanced => 1.7,
            FsrMode::Performance => 2.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FsrMode::Off => "FSR: Off",
            FsrMode::UltraQuality => "FSR: Ultra Quality (1.3x)",
            FsrMode::Quality => "FSR: Quality (1.5x)",
            FsrMode::Balanced => "FSR: Balanced (1.7x)",
            FsrMode::Performance => "FSR: Performance (2.0x)",
        }
    }

    pub fn next(self) -> Self {
        match self {
            FsrMode::Off => FsrMode::UltraQuality,
            FsrMode::UltraQuality => FsrMode::Quality,
            FsrMode::Quality => FsrMode::Balanced,
            FsrMode::Balanced => FsrMode::Performance,
            FsrMode::Performance => FsrMode::Off,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WorldPushConstants {
    pub mvp: [f32; 16],
    pub model: [f32; 16],
    pub tint_color: [f32; 4],
    pub map_params: [f32; 4],
}

pub const MAX_RT_BOXES: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RtBox {
    pub center_y_half: [f32; 4],
    pub half_xz_yaw: [f32; 4],
    pub color_meta: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RtSceneGpu {
    pub count_and_cam: [f32; 4],
    pub boxes: [RtBox; MAX_RT_BOXES],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UiPushConstants {
    pub screen_size: [f32; 2],
    pub use_texture: f32,
    pub _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FsrEasuConstants {
    pub src_size: [f32; 2],
    pub dst_size: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FsrRcasConstants {
    pub dst_size: [f32; 2],
    pub sharpness: f32,
    pub _pad: f32,
}

pub struct VulkanBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
}

pub struct VulkanTexture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub width: u32,
    pub height: u32,
}

#[allow(dead_code)]
pub struct VulkanRenderer {
    _entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    _queue_family_index: u32,

    surface_loader: Option<ash::khr::surface::Instance>,
    surface: Option<vk::SurfaceKHR>,
    swapchain_loader: Option<ash::khr::swapchain::Device>,
    swapchain: Option<vk::SwapchainKHR>,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,

    depth_image: vk::Image,
    depth_memory: vk::DeviceMemory,
    depth_view: vk::ImageView,

    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,

    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,

    // Fog texture & optional Ray Tracing SSBO
    fog_texture: VulkanTexture,
    fog_image_layout: vk::ImageLayout,
    fog_staging_buffer: VulkanBuffer,
    rt_scene_buffer: VulkanBuffer,
    pub rt_enabled: bool,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,

    // Pipelines
    world_pipeline_layout: vk::PipelineLayout,
    world_pipeline: vk::Pipeline,
    ui_pipeline_layout: vk::PipelineLayout,
    ui_pipeline: vk::Pipeline,

    // AMD FSR 1.0 (EASU & RCAS)
    pub fsr_mode: FsrMode,
    fsr_easu_pipeline_layout: vk::PipelineLayout,
    fsr_easu_pipeline: vk::Pipeline,
    fsr_rcas_pipeline_layout: vk::PipelineLayout,
    fsr_rcas_pipeline: vk::Pipeline,

    // Meshes & Geometry
    pub mesh_library: MeshLibrary,
    mesh_vertex_buffer: VulkanBuffer,
    mesh_index_buffer: VulkanBuffer,

    // Dynamic UI vertex buffer
    ui_vertex_buffer: VulkanBuffer,
    ui_vertex_capacity: usize,

    is_headless: bool,
}

impl VulkanRenderer {
    pub fn new(
        x11_window: Option<(*mut libc::c_void, libc::c_ulong)>,
        initial_width: u32,
        initial_height: u32,
    ) -> Result<Self, String> {
        let entry = unsafe {
            ash::Entry::load().map_err(|e| format!("Failed to dynamically load Vulkan: {:?}", e))?
        };

        let app_name = std::ffi::CString::new("Midnight Battle Zone").unwrap();
        let engine_name = std::ffi::CString::new("MBZ Engine").unwrap();
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::make_api_version(0, 1, 2, 0));

        let mut instance_extensions: Vec<*const libc::c_char> = Vec::new();
        let is_headless = x11_window.is_none();

        if !is_headless {
            instance_extensions.push(ash::khr::surface::NAME.as_ptr());
            instance_extensions.push(ash::khr::xlib_surface::NAME.as_ptr());
        }

        let instance_ci = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions);

        let instance = unsafe {
            entry
                .create_instance(&instance_ci, None)
                .map_err(|e| format!("Failed to create Vulkan instance: {:?}", e))?
        };

        // Surface setup
        let (surface_loader, surface) = if let Some((display, window)) = x11_window {
            let surf_loader = ash::khr::surface::Instance::new(&entry, &instance);
            let xlib_loader = ash::khr::xlib_surface::Instance::new(&entry, &instance);
            let xlib_ci = vk::XlibSurfaceCreateInfoKHR::default()
                .dpy(display as *mut _)
                .window(window as _);
            let surf = unsafe {
                xlib_loader
                    .create_xlib_surface(&xlib_ci, None)
                    .map_err(|e| format!("Failed to create X11 Vulkan surface: {:?}", e))?
            };
            (Some(surf_loader), Some(surf))
        } else {
            (None, None)
        };

        // Physical Device selection
        let pdevices = unsafe {
            instance
                .enumerate_physical_devices()
                .map_err(|e| format!("Failed to enumerate physical devices: {:?}", e))?
        };

        if pdevices.is_empty() {
            return Err("No Vulkan physical devices found on system".to_string());
        }

        let mut best_pdev = None;
        let mut best_score = -1;
        let mut best_q_family = 0;

        for &pdev in &pdevices {
            let props = unsafe { instance.get_physical_device_properties(pdev) };
            let q_props = unsafe { instance.get_physical_device_queue_family_properties(pdev) };

            let mut q_family_opt = None;
            for (idx, q) in q_props.iter().enumerate() {
                if q.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    if let (Some(surf_loader), Some(surf)) = (&surface_loader, surface) {
                        let present_ok = unsafe {
                            surf_loader
                                .get_physical_device_surface_support(pdev, idx as u32, surf)
                                .unwrap_or(false)
                        };
                        if present_ok {
                            q_family_opt = Some(idx as u32);
                            break;
                        }
                    } else {
                        q_family_opt = Some(idx as u32);
                        break;
                    }
                }
            }

            if let Some(q_family) = q_family_opt {
                let score = match props.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 4000,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 3000,
                    vk::PhysicalDeviceType::VIRTUAL_GPU => 2000,
                    vk::PhysicalDeviceType::CPU => 1000,
                    _ => 500,
                };
                if score > best_score {
                    best_score = score;
                    best_pdev = Some(pdev);
                    best_q_family = q_family;
                }
            }
        }

        let physical_device = best_pdev.ok_or("No suitable Vulkan physical device found")?;
        let queue_family_index = best_q_family;

        let dev_props = unsafe { instance.get_physical_device_properties(physical_device) };
        let dev_name = unsafe { CStr::from_ptr(dev_props.device_name.as_ptr()) };
        log::info!("Selected Vulkan device: {:?}", dev_name);

        // Logical Device
        let queue_priorities = [1.0f32];
        let queue_ci = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities)];

        let mut device_extensions = Vec::new();
        if surface.is_some() {
            device_extensions.push(ash::khr::swapchain::NAME.as_ptr());
        }

        let device_ci = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_ci)
            .enabled_extension_names(&device_extensions);

        let device = unsafe {
            instance
                .create_device(physical_device, &device_ci, None)
                .map_err(|e| format!("Failed to create logical device: {:?}", e))?
        };

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };

        // Swapchain
        let swapchain_extent = vk::Extent2D {
            width: initial_width.max(64),
            height: initial_height.max(64),
        };
        let mut swapchain_loader_opt = None;
        let mut swapchain_opt = None;
        let mut swapchain_images = Vec::new();
        let mut swapchain_image_views = Vec::new();
        let swapchain_format = vk::Format::B8G8R8A8_UNORM;

        if let (Some(surf_loader), Some(surf)) = (&surface_loader, surface) {
            let surf_caps = unsafe {
                surf_loader
                    .get_physical_device_surface_capabilities(physical_device, surf)
                    .map_err(|e| format!("Surface caps query failed: {:?}", e))?
            };

            let mut image_count = surf_caps.min_image_count + 1;
            if surf_caps.max_image_count > 0 && image_count > surf_caps.max_image_count {
                image_count = surf_caps.max_image_count;
            }

            let swapchain_ci = vk::SwapchainCreateInfoKHR::default()
                .surface(surf)
                .min_image_count(image_count)
                .image_format(swapchain_format)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_extent(swapchain_extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(surf_caps.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(vk::PresentModeKHR::FIFO)
                .clipped(true);

            let sw_loader = ash::khr::swapchain::Device::new(&instance, &device);
            let sw = unsafe {
                sw_loader
                    .create_swapchain(&swapchain_ci, None)
                    .map_err(|e| format!("Swapchain creation failed: {:?}", e))?
            };

            let images = unsafe {
                sw_loader
                    .get_swapchain_images(sw)
                    .map_err(|e| format!("Failed to get swapchain images: {:?}", e))?
            };

            for &img in &images {
                let view_ci = vk::ImageViewCreateInfo::default()
                    .image(img)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(swapchain_format)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    );
                let view = unsafe {
                    device
                        .create_image_view(&view_ci, None)
                        .map_err(|e| format!("Failed to create swapchain image view: {:?}", e))?
                };
                swapchain_image_views.push(view);
            }

            swapchain_images = images;
            swapchain_loader_opt = Some(sw_loader);
            swapchain_opt = Some(sw);
        }

        // Depth Buffer (D32_SFLOAT)
        let depth_format = vk::Format::D32_SFLOAT;
        let depth_img_ci = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(depth_format)
            .extent(vk::Extent3D {
                width: swapchain_extent.width,
                height: swapchain_extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let depth_image = unsafe {
            device
                .create_image(&depth_img_ci, None)
                .map_err(|e| format!("Failed to create depth image: {:?}", e))?
        };

        let depth_req = unsafe { device.get_image_memory_requirements(depth_image) };
        let depth_mem_type = find_memory_type(
            &mem_props,
            depth_req.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .ok_or("Failed to find device-local memory for depth buffer")?;

        let depth_alloc_ci = vk::MemoryAllocateInfo::default()
            .allocation_size(depth_req.size)
            .memory_type_index(depth_mem_type);

        let depth_memory = unsafe {
            device
                .allocate_memory(&depth_alloc_ci, None)
                .map_err(|e| format!("Failed to allocate depth memory: {:?}", e))?
        };

        unsafe {
            device
                .bind_image_memory(depth_image, depth_memory, 0)
                .map_err(|e| format!("Failed to bind depth memory: {:?}", e))?;
        }

        let depth_view_ci = vk::ImageViewCreateInfo::default()
            .image(depth_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(depth_format)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

        let depth_view = unsafe {
            device
                .create_image_view(&depth_view_ci, None)
                .map_err(|e| format!("Failed to create depth view: {:?}", e))?
        };

        // RenderPass
        let color_attachment = vk::AttachmentDescription::default()
            .format(swapchain_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(if is_headless {
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
            } else {
                vk::ImageLayout::PRESENT_SRC_KHR
            });

        let depth_attachment = vk::AttachmentDescription::default()
            .format(depth_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let color_ref = [vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

        let depth_ref = vk::AttachmentReference::default()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let subpass = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_ref)
            .depth_stencil_attachment(&depth_ref)];

        let dependency = [vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .dst_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            )];

        let attachments = [color_attachment, depth_attachment];
        let render_pass_ci = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpass)
            .dependencies(&dependency);

        let render_pass = unsafe {
            device
                .create_render_pass(&render_pass_ci, None)
                .map_err(|e| format!("Failed to create render pass: {:?}", e))?
        };

        // Framebuffers
        let mut framebuffers = Vec::new();
        for &color_view in &swapchain_image_views {
            let attachments = [color_view, depth_view];
            let fb_ci = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(swapchain_extent.width)
                .height(swapchain_extent.height)
                .layers(1);
            let fb = unsafe {
                device
                    .create_framebuffer(&fb_ci, None)
                    .map_err(|e| format!("Failed to create framebuffer: {:?}", e))?
            };
            framebuffers.push(fb);
        }

        // Command Pool & Buffer
        let cmd_pool_ci = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = unsafe {
            device
                .create_command_pool(&cmd_pool_ci, None)
                .map_err(|e| format!("Failed to create command pool: {:?}", e))?
        };

        let cmd_alloc_ci = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe {
            device
                .allocate_command_buffers(&cmd_alloc_ci)
                .map_err(|e| format!("Failed to allocate command buffer: {:?}", e))?[0]
        };

        // Sync primitives
        let sema_ci = vk::SemaphoreCreateInfo::default();
        let fence_ci = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let image_available_semaphore = unsafe { device.create_semaphore(&sema_ci, None).unwrap() };
        let render_finished_semaphore = unsafe { device.create_semaphore(&sema_ci, None).unwrap() };
        let in_flight_fence = unsafe { device.create_fence(&fence_ci, None).unwrap() };

        // Fog of War 2D Texture (R8G8B8A8_UNORM)
        let fog_w = 64u32;
        let fog_h = 64u32;
        let fog_texture = create_vulkan_texture(
            &device,
            &mem_props,
            fog_w,
            fog_h,
            vk::Format::R8G8B8A8_UNORM,
        )?;

        let fog_staging_buffer = create_vulkan_buffer(
            &device,
            &mem_props,
            (fog_w * fog_h * 4) as vk::DeviceSize,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Descriptor Set Layout
        let rt_scene_buffer = create_vulkan_buffer(
            &device,
            &mem_props,
            std::mem::size_of::<RtSceneGpu>() as vk::DeviceSize,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let desc_bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];

        let desc_layout_ci =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&desc_bindings);

        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&desc_layout_ci, None)
                .map_err(|e| format!("Failed to create descriptor set layout: {:?}", e))?
        };

        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(8),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(8),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(8),
        ];

        let pool_ci = vk::DescriptorPoolCreateInfo::default()
            .max_sets(8)
            .pool_sizes(&pool_sizes);

        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&pool_ci, None)
                .map_err(|e| format!("Failed to create descriptor pool: {:?}", e))?
        };

        let set_layouts = [descriptor_set_layout];
        let set_alloc_ci = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);

        let descriptor_set = unsafe {
            device
                .allocate_descriptor_sets(&set_alloc_ci)
                .map_err(|e| format!("Failed to allocate descriptor set: {:?}", e))?[0]
        };

        let image_info = [vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(fog_texture.view)];

        let sampler_info = [vk::DescriptorImageInfo::default().sampler(fog_texture.sampler)];

        let rt_buf_info = [vk::DescriptorBufferInfo::default()
            .buffer(rt_scene_buffer.buffer)
            .offset(0)
            .range(std::mem::size_of::<RtSceneGpu>() as vk::DeviceSize)];

        let descriptor_writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&sampler_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&rt_buf_info),
        ];

        unsafe {
            device.update_descriptor_sets(&descriptor_writes, &[]);
        }

        // Shaders & Pipelines (World, UI, and AMD FSR EASU/RCAS)
        let shader_bins = shaders::compile_all_shaders()
            .map_err(|e| format!("Shader compile error: {}", e))?;

        let world_module = create_shader_module(&device, &shader_bins.world_spv)?;
        let ui_module = create_shader_module(&device, &shader_bins.ui_spv)?;
        let fsr_easu_module = create_shader_module(&device, &shader_bins.fsr_easu_spv)?;
        let fsr_rcas_module = create_shader_module(&device, &shader_bins.fsr_rcas_spv)?;

        // Pipeline Layouts
        let world_pc_range = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<WorldPushConstants>() as u32)];

        let world_pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&world_pc_range);

        let world_pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&world_pipeline_layout_ci, None)
                .map_err(|e| format!("World pipeline layout failed: {:?}", e))?
        };

        let ui_pc_range = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<UiPushConstants>() as u32)];

        let ui_pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&ui_pc_range);

        let ui_pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&ui_pipeline_layout_ci, None)
                .map_err(|e| format!("UI pipeline layout failed: {:?}", e))?
        };

        // FSR EASU & RCAS Pipeline Layouts
        let easu_pc_range = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<FsrEasuConstants>() as u32)];

        let fsr_easu_pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&easu_pc_range);

        let fsr_easu_pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&fsr_easu_pipeline_layout_ci, None)
                .map_err(|e| format!("FSR EASU layout failed: {:?}", e))?
        };

        let rcas_pc_range = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<FsrRcasConstants>() as u32)];

        let fsr_rcas_pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&rcas_pc_range);

        let fsr_rcas_pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&fsr_rcas_pipeline_layout_ci, None)
                .map_err(|e| format!("FSR RCAS layout failed: {:?}", e))?
        };

        // 3D World Graphics Pipeline
        let world_pipeline = create_world_pipeline(
            &device,
            render_pass,
            world_pipeline_layout,
            world_module,
            swapchain_extent,
        )?;

        // 2D UI Graphics Pipeline
        let ui_pipeline = create_ui_pipeline(
            &device,
            render_pass,
            ui_pipeline_layout,
            ui_module,
            swapchain_extent,
        )?;

        // FSR EASU & RCAS Graphics Pipelines
        let fsr_easu_pipeline = create_fullscreen_post_pipeline(
            &device,
            render_pass,
            fsr_easu_pipeline_layout,
            fsr_easu_module,
            swapchain_extent,
        )?;

        let fsr_rcas_pipeline = create_fullscreen_post_pipeline(
            &device,
            render_pass,
            fsr_rcas_pipeline_layout,
            fsr_rcas_module,
            swapchain_extent,
        )?;

        // Mesh library & GPU Buffers
        let mesh_library = MeshLibrary::new();

        let v_size = (mesh_library.combined_vertices.len() * std::mem::size_of::<Vertex3D>())
            as vk::DeviceSize;
        let mesh_vertex_buffer = create_vulkan_buffer(
            &device,
            &mem_props,
            v_size.max(64),
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let ptr = device
                .map_memory(mesh_vertex_buffer.memory, 0, v_size, vk::MemoryMapFlags::empty())
                .unwrap();
            std::ptr::copy_nonoverlapping(
                mesh_library.combined_vertices.as_ptr() as *const u8,
                ptr as *mut u8,
                v_size as usize,
            );
            device.unmap_memory(mesh_vertex_buffer.memory);
        }

        let i_size = (mesh_library.combined_indices.len() * std::mem::size_of::<u32>())
            as vk::DeviceSize;
        let mesh_index_buffer = create_vulkan_buffer(
            &device,
            &mem_props,
            i_size.max(64),
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let ptr = device
                .map_memory(mesh_index_buffer.memory, 0, i_size, vk::MemoryMapFlags::empty())
                .unwrap();
            std::ptr::copy_nonoverlapping(
                mesh_library.combined_indices.as_ptr() as *const u8,
                ptr as *mut u8,
                i_size as usize,
            );
            device.unmap_memory(mesh_index_buffer.memory);
        }

        // Dynamic 2D UI Vertex Buffer
        let ui_vertex_capacity = 262144;
        let ui_v_size = (ui_vertex_capacity * std::mem::size_of::<Vertex2D>()) as vk::DeviceSize;
        let ui_vertex_buffer = create_vulkan_buffer(
            &device,
            &mem_props,
            ui_v_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Destroy temporary shader modules
        unsafe {
            device.destroy_shader_module(world_module, None);
            device.destroy_shader_module(ui_module, None);
            device.destroy_shader_module(fsr_easu_module, None);
            device.destroy_shader_module(fsr_rcas_module, None);
        }

        Ok(Self {
            _entry: entry,
            instance,
            physical_device,
            device,
            queue,
            _queue_family_index: queue_family_index,
            surface_loader,
            surface,
            swapchain_loader: swapchain_loader_opt,
            swapchain: swapchain_opt,
            swapchain_images,
            swapchain_image_views,
            swapchain_format,
            swapchain_extent,
            depth_image,
            depth_memory,
            depth_view,
            render_pass,
            framebuffers,
            command_pool,
            command_buffer,
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
            fog_texture,
            fog_image_layout: vk::ImageLayout::UNDEFINED,
            fog_staging_buffer,
            rt_scene_buffer,
            rt_enabled: false,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
            world_pipeline_layout,
            world_pipeline,
            ui_pipeline_layout,
            ui_pipeline,
            fsr_mode: FsrMode::Off,
            fsr_easu_pipeline_layout,
            fsr_easu_pipeline,
            fsr_rcas_pipeline_layout,
            fsr_rcas_pipeline,
            mesh_library,
            mesh_vertex_buffer,
            mesh_index_buffer,
            ui_vertex_buffer,
            ui_vertex_capacity,
            is_headless,
        })
    }

    pub fn set_fsr_mode(&mut self, mode: FsrMode) {
        self.fsr_mode = mode;
        log::info!("FSR Mode changed to: {:?}", mode.label());
    }

    pub fn set_rt_enabled(&mut self, enabled: bool) {
        self.rt_enabled = enabled;
        log::info!(
            "Optional Ray Tracing {}",
            if enabled { "ENABLED" } else { "DISABLED" }
        );
    }

    pub fn update_rt_scene(&mut self, camera: &IsometricCamera, instances: &[RenderInstance]) {
        if !self.rt_enabled {
            return;
        }
        let cam_pos = camera.eye_position();
        let mut boxes = [RtBox::default(); MAX_RT_BOXES];
        let mut count = 0usize;

        for inst in instances {
            if count >= MAX_RT_BOXES {
                break;
            }
            if inst.mesh_archetype == "projectile" || inst.mesh_archetype == "selection_ring" {
                continue;
            }
            let (base_half_xz, base_h, is_emissive) = match inst.mesh_archetype.as_str() {
                "command_center" => (2.15, 3.2, 0.0),
                "barracks" => (1.55, 1.5, 0.0),
                "mine" => (1.15, 2.6, 0.0),
                "house" => (1.05, 1.8, 0.0),
                "turret" => (1.35, 2.0, 0.0),
                "obstacle" => (1.05, 2.3, 0.0),
                "resource_node" => (0.85, 2.1, 1.0),
                _ => (0.45, 1.0, 0.0),
            };
            let hx = (base_half_xz * inst.scale.x).max(0.25);
            let hy = (base_h * inst.scale.y * 0.5).max(0.35);
            let hz = (base_half_xz * inst.scale.z).max(0.25);
            let cy = (-inst.rotation_yaw).cos();
            let sy = (-inst.rotation_yaw).sin();

            boxes[count] = RtBox {
                center_y_half: [inst.position.x, inst.position.y + hy, inst.position.z, hy],
                half_xz_yaw: [hx, hz, cy, sy],
                color_meta: [inst.color[0], inst.color[1], inst.color[2], is_emissive],
            };
            count += 1;
        }

        let gpu_scene = RtSceneGpu {
            count_and_cam: [count as f32, cam_pos.x, cam_pos.y, cam_pos.z],
            boxes,
        };
        let bytes_len = std::mem::size_of::<RtSceneGpu>();
        unsafe {
            if let Ok(ptr) = self.device.map_memory(
                self.rt_scene_buffer.memory,
                0,
                bytes_len as vk::DeviceSize,
                vk::MemoryMapFlags::empty(),
            ) {
                std::ptr::copy_nonoverlapping(
                    &gpu_scene as *const RtSceneGpu as *const u8,
                    ptr as *mut u8,
                    bytes_len,
                );
                self.device.unmap_memory(self.rt_scene_buffer.memory);
            }
        }
    }

    pub fn toggle_fsr(&mut self) -> FsrMode {
        let next = self.fsr_mode.next();
        self.set_fsr_mode(next);
        next
    }

    pub fn handle_resize(&mut self, new_width: u32, new_height: u32) {
        let w = new_width.max(64);
        let h = new_height.max(64);
        if w == self.swapchain_extent.width && h == self.swapchain_extent.height {
            return;
        }
        if self.is_headless {
            self.swapchain_extent = vk::Extent2D { width: w, height: h };
            return;
        }

        let (Some(surf_loader), Some(surf), Some(sw_loader)) = (
            &self.surface_loader,
            self.surface,
            &self.swapchain_loader,
        ) else {
            return;
        };

        unsafe {
            let _ = self.device.device_wait_idle();

            for &fb in &self.framebuffers {
                self.device.destroy_framebuffer(fb, None);
            }
            self.framebuffers.clear();

            if self.depth_view != vk::ImageView::null() {
                self.device.destroy_image_view(self.depth_view, None);
            }
            if self.depth_image != vk::Image::null() {
                self.device.destroy_image(self.depth_image, None);
                self.device.free_memory(self.depth_memory, None);
            }

            for &view in &self.swapchain_image_views {
                self.device.destroy_image_view(view, None);
            }
            self.swapchain_image_views.clear();

            if let Some(old_sw) = self.swapchain.take() {
                sw_loader.destroy_swapchain(old_sw, None);
            }

            let Ok(surf_caps) = surf_loader
                .get_physical_device_surface_capabilities(self.physical_device, surf)
            else {
                return;
            };

            let clamped_w = w.clamp(
                surf_caps.min_image_extent.width.max(64),
                surf_caps.max_image_extent.width.max(w),
            );
            let clamped_h = h.clamp(
                surf_caps.min_image_extent.height.max(64),
                surf_caps.max_image_extent.height.max(h),
            );
            self.swapchain_extent = vk::Extent2D {
                width: clamped_w,
                height: clamped_h,
            };

            let mut image_count = surf_caps.min_image_count + 1;
            if surf_caps.max_image_count > 0 && image_count > surf_caps.max_image_count {
                image_count = surf_caps.max_image_count;
            }

            let swapchain_ci = vk::SwapchainCreateInfoKHR::default()
                .surface(surf)
                .min_image_count(image_count)
                .image_format(self.swapchain_format)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_extent(self.swapchain_extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(surf_caps.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(vk::PresentModeKHR::FIFO)
                .clipped(true);

            let Ok(sw) = sw_loader.create_swapchain(&swapchain_ci, None) else {
                return;
            };
            let Ok(images) = sw_loader.get_swapchain_images(sw) else {
                return;
            };

            for &img in &images {
                let view_ci = vk::ImageViewCreateInfo::default()
                    .image(img)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(self.swapchain_format)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    );
                if let Ok(view) = self.device.create_image_view(&view_ci, None) {
                    self.swapchain_image_views.push(view);
                }
            }
            self.swapchain_images = images;
            self.swapchain = Some(sw);

            let mem_props = self
                .instance
                .get_physical_device_memory_properties(self.physical_device);
            let depth_format = vk::Format::D32_SFLOAT;
            let depth_img_ci = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(depth_format)
                .extent(vk::Extent3D {
                    width: self.swapchain_extent.width,
                    height: self.swapchain_extent.height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED);

            if let Ok(d_img) = self.device.create_image(&depth_img_ci, None) {
                self.depth_image = d_img;
                let depth_req = self.device.get_image_memory_requirements(d_img);
                if let Some(d_mem_type) = find_memory_type(
                    &mem_props,
                    depth_req.memory_type_bits,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                ) {
                    let alloc_ci = vk::MemoryAllocateInfo::default()
                        .allocation_size(depth_req.size)
                        .memory_type_index(d_mem_type);
                    if let Ok(d_mem) = self.device.allocate_memory(&alloc_ci, None) {
                        self.depth_memory = d_mem;
                        let _ = self.device.bind_image_memory(d_img, d_mem, 0);
                        let depth_view_ci = vk::ImageViewCreateInfo::default()
                            .image(d_img)
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(depth_format)
                            .subresource_range(
                                vk::ImageSubresourceRange::default()
                                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                                    .base_mip_level(0)
                                    .level_count(1)
                                    .base_array_layer(0)
                                    .layer_count(1),
                            );
                        if let Ok(d_view) = self.device.create_image_view(&depth_view_ci, None) {
                            self.depth_view = d_view;
                        }
                    }
                }
            }

            for &color_view in &self.swapchain_image_views {
                let attachments = [color_view, self.depth_view];
                let fb_ci = vk::FramebufferCreateInfo::default()
                    .render_pass(self.render_pass)
                    .attachments(&attachments)
                    .width(self.swapchain_extent.width)
                    .height(self.swapchain_extent.height)
                    .layers(1);
                if let Ok(fb) = self.device.create_framebuffer(&fb_ci, None) {
                    self.framebuffers.push(fb);
                }
            }
        }
    }

    pub fn update_fog_texture(&mut self, fog_rgba: &[u8], fog_dims: (u32, u32)) {
        let dst_w = self.fog_texture.width.max(1) as usize;
        let dst_h = self.fog_texture.height.max(1) as usize;
        let src_w = fog_dims.0 as usize;
        let src_h = fog_dims.1 as usize;

        let mut resampled = vec![255u8; dst_w * dst_h * 4];
        if !fog_rgba.is_empty() && src_w > 0 && src_h > 0 {
            for y in 0..dst_h {
                let sy = ((y * src_h) / dst_h).min(src_h - 1);
                for x in 0..dst_w {
                    let sx = ((x * src_w) / dst_w).min(src_w - 1);
                    let src_idx = (sy * src_w + sx) * 4;
                    let dst_idx = (y * dst_w + x) * 4;
                    if src_idx + 4 <= fog_rgba.len() {
                        resampled[dst_idx..dst_idx + 4]
                            .copy_from_slice(&fog_rgba[src_idx..src_idx + 4]);
                    }
                }
            }
        }

        let copy_len = resampled.len().min(self.fog_staging_buffer.size as usize);

        unsafe {
            let ptr = self
                .device
                .map_memory(
                    self.fog_staging_buffer.memory,
                    0,
                    self.fog_staging_buffer.size,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            std::ptr::copy_nonoverlapping(resampled.as_ptr(), ptr as *mut u8, copy_len);
            self.device.unmap_memory(self.fog_staging_buffer.memory);
        }
    }

    pub fn render_frame(
        &mut self,
        camera: &IsometricCamera,
        map_size: Vec2,
        fog_rgba: &[u8],
        fog_dims: (u32, u32),
        instances: &[RenderInstance],
        ui_vertices: &[Vertex2D],
    ) -> Result<(), String> {
        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight_fence], true, u64::MAX)
                .map_err(|e| format!("Fence wait failed: {:?}", e))?;
        }

        let image_index = if let (Some(sw_loader), Some(sw)) =
            (&self.swapchain_loader, self.swapchain)
        {
            let (idx, _suboptimal) = unsafe {
                sw_loader
                    .acquire_next_image(
                        sw,
                        u64::MAX,
                        self.image_available_semaphore,
                        vk::Fence::null(),
                    )
                    .map_err(|e| format!("Acquire next image failed: {:?}", e))?
            };
            idx as usize
        } else {
            0
        };

        if image_index >= self.framebuffers.len() {
            return Ok(());
        }

        unsafe {
            self.device
                .reset_fences(&[self.in_flight_fence])
                .map_err(|e| format!("Fence reset failed: {:?}", e))?;
        }

        self.update_fog_texture(fog_rgba, fog_dims);
        self.update_rt_scene(camera, instances);

        // Upload UI vertices
        let ui_count = ui_vertices.len().min(self.ui_vertex_capacity);
        if ui_count > 0 {
            let upload_bytes = (ui_count * std::mem::size_of::<Vertex2D>()) as vk::DeviceSize;
            unsafe {
                let ptr = self
                    .device
                    .map_memory(
                        self.ui_vertex_buffer.memory,
                        0,
                        upload_bytes,
                        vk::MemoryMapFlags::empty(),
                    )
                    .unwrap();
                std::ptr::copy_nonoverlapping(
                    ui_vertices.as_ptr() as *const u8,
                    ptr as *mut u8,
                    upload_bytes as usize,
                );
                self.device.unmap_memory(self.ui_vertex_buffer.memory);
            }
        }

        let cmd = self.command_buffer;
        unsafe {
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .unwrap();

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(cmd, &begin_info).unwrap();

            // Copy Fog of War staging buffer to GPU fog_texture.image
            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let (fog_src_stage, fog_src_access) = if self.fog_image_layout
                == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            {
                (vk::PipelineStageFlags::FRAGMENT_SHADER, vk::AccessFlags::SHADER_READ)
            } else {
                (vk::PipelineStageFlags::TOP_OF_PIPE, vk::AccessFlags::empty())
            };
            let barrier_to_dst = [vk::ImageMemoryBarrier::default()
                .old_layout(self.fog_image_layout)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_access_mask(fog_src_access)
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .image(self.fog_texture.image)
                .subresource_range(subresource_range)];

            self.device.cmd_pipeline_barrier(
                cmd,
                fog_src_stage,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barrier_to_dst,
            );

            let copy_region = [vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1),
                )
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width: self.fog_texture.width,
                    height: self.fog_texture.height,
                    depth: 1,
                })];

            self.device.cmd_copy_buffer_to_image(
                cmd,
                self.fog_staging_buffer.buffer,
                self.fog_texture.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &copy_region,
            );

            let barrier_to_shader = [vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .image(self.fog_texture.image)
                .subresource_range(subresource_range)];

            self.device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barrier_to_shader,
            );

            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.07, 0.11, 0.16, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[image_index])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_extent,
                })
                .clear_values(&clear_values);

            self.device.cmd_begin_render_pass(
                cmd,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );

            // Compute dynamic viewport scale based on FSR scale factor
            let fsr_scale = self.fsr_mode.scale_factor();
            let render_w = (self.swapchain_extent.width as f32 / fsr_scale).max(64.0);
            let render_h = (self.swapchain_extent.height as f32 / fsr_scale).max(64.0);

            let viewport = [vk::Viewport::default()
                .x(0.0)
                .y(0.0)
                .width(render_w)
                .height(render_h)
                .min_depth(0.0)
                .max_depth(1.0)];
            self.device.cmd_set_viewport(cmd, 0, &viewport);

            let scissor = [vk::Rect2D::default()
                .offset(vk::Offset2D { x: 0, y: 0 })
                .extent(vk::Extent2D {
                    width: render_w as u32,
                    height: render_h as u32,
                })];
            self.device.cmd_set_scissor(cmd, 0, &scissor);

            // 1. Draw 3D World (Terrain + Units/Buildings)
            self.device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.world_pipeline);
            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.world_pipeline_layout,
                0,
                &[self.descriptor_set],
                &[],
            );
            self.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.mesh_vertex_buffer.buffer],
                &[0],
            );
            self.device.cmd_bind_index_buffer(
                cmd,
                self.mesh_index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );

            let aspect = render_w / render_h.max(1.0);
            let vp = camera.view_proj_matrix(aspect);
            let rt_flag = if self.rt_enabled { 1.0 } else { 0.0 };

            // Draw Terrain
            if let Some(terrain_range) = self.mesh_library.get_range("terrain") {
                let terrain_model = Mat4::from_scale(Vec3::new(map_size.x, 1.0, map_size.y));
                let terrain_mvp = vp * terrain_model;
                let pc = WorldPushConstants {
                    mvp: terrain_mvp.to_cols_array(),
                    model: terrain_model.to_cols_array(),
                    tint_color: [1.0, 1.0, 1.0, 0.0],
                    map_params: [map_size.x, map_size.y, rt_flag, 0.0],
                };

                let pc_bytes = std::slice::from_raw_parts(
                    &pc as *const _ as *const u8,
                    std::mem::size_of::<WorldPushConstants>(),
                );
                self.device.cmd_push_constants(
                    cmd,
                    self.world_pipeline_layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    pc_bytes,
                );

                self.device.cmd_draw_indexed(
                    cmd,
                    terrain_range.index_count,
                    1,
                    terrain_range.index_start,
                    0,
                    0,
                );
            }

            // Draw 3D Instances
            for inst in instances {
                if let Some(range) = self.mesh_library.get_range(&inst.mesh_archetype) {
                    let rot_y = Mat4::from_rotation_y(inst.rotation_yaw);
                    let scale = Mat4::from_scale(inst.scale);
                    let trans = Mat4::from_translation(inst.position);
                    let model = trans * rot_y * scale;
                    let mvp = vp * model;

                    let tex_mode = match inst.mesh_archetype.as_str() {
                        "command_center" | "mine" | "turret" => 1.0,
                        "house" => 2.0,
                        "barracks" => 3.0,
                        "obstacle" => 4.0,
                        _ => 0.0,
                    };

                    let pc = WorldPushConstants {
                        mvp: mvp.to_cols_array(),
                        model: model.to_cols_array(),
                        tint_color: inst.color,
                        map_params: [map_size.x, map_size.y, rt_flag, tex_mode],
                    };

                    let pc_bytes = std::slice::from_raw_parts(
                        &pc as *const _ as *const u8,
                        std::mem::size_of::<WorldPushConstants>(),
                    );
                    self.device.cmd_push_constants(
                        cmd,
                        self.world_pipeline_layout,
                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                        0,
                        pc_bytes,
                    );

                    self.device.cmd_draw_indexed(
                        cmd,
                        range.index_count,
                        1,
                        range.index_start,
                        0,
                        0,
                    );
                }

                // Draw team indicator ring under Cat units or bright green selection ring when selected
                let is_cat_unit = matches!(
                    inst.mesh_archetype.as_str(),
                    "worker" | "soldier" | "sniper" | "heavy"
                );
                if inst.selected || is_cat_unit {
                    if let Some(ring_range) = self.mesh_library.get_range("selection_ring") {
                        let base_r = if inst.mesh_archetype == "command_center" {
                            2.45
                        } else if matches!(
                            inst.mesh_archetype.as_str(),
                            "house" | "mine" | "barracks" | "turret" | "resource_node"
                        ) {
                            1.50
                        } else if inst.selected {
                            1.05
                        } else {
                            0.72
                        };
                        let ring_scale = Vec3::new(base_r, 1.0, base_r);
                        let ring_pos = Vec3::new(inst.position.x, 0.03, inst.position.z);
                        let model = Mat4::from_translation(ring_pos) * Mat4::from_scale(ring_scale);
                        let mvp = vp * model;

                        let ring_tint = if inst.selected {
                            [0.2, 1.0, 0.3, 0.92]
                        } else {
                            [inst.color[0], inst.color[1], inst.color[2], 0.92]
                        };

                        let pc = WorldPushConstants {
                            mvp: mvp.to_cols_array(),
                            model: model.to_cols_array(),
                            tint_color: ring_tint,
                            map_params: [map_size.x, map_size.y, 0.0, 0.0],
                        };

                        let pc_bytes = std::slice::from_raw_parts(
                            &pc as *const _ as *const u8,
                            std::mem::size_of::<WorldPushConstants>(),
                        );
                        self.device.cmd_push_constants(
                            cmd,
                            self.world_pipeline_layout,
                            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                            0,
                            pc_bytes,
                        );

                        self.device.cmd_draw_indexed(
                            cmd,
                            ring_range.index_count,
                            1,
                            ring_range.index_start,
                            0,
                            0,
                        );
                    }
                }
            }

            // If FSR enabled: apply EASU and RCAS sharpening
            if self.fsr_mode != FsrMode::Off {
                let full_viewport = [vk::Viewport::default()
                    .x(0.0)
                    .y(0.0)
                    .width(self.swapchain_extent.width as f32)
                    .height(self.swapchain_extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0)];
                self.device.cmd_set_viewport(cmd, 0, &full_viewport);

                let full_scissor = [vk::Rect2D::default()
                    .offset(vk::Offset2D { x: 0, y: 0 })
                    .extent(self.swapchain_extent)];
                self.device.cmd_set_scissor(cmd, 0, &full_scissor);

                // Run FSR EASU
                self.device.cmd_bind_pipeline(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.fsr_easu_pipeline,
                );
                let easu_pc = FsrEasuConstants {
                    src_size: [render_w, render_h],
                    dst_size: [
                        self.swapchain_extent.width as f32,
                        self.swapchain_extent.height as f32,
                    ],
                };
                let easu_bytes = std::slice::from_raw_parts(
                    &easu_pc as *const _ as *const u8,
                    std::mem::size_of::<FsrEasuConstants>(),
                );
                self.device.cmd_push_constants(
                    cmd,
                    self.fsr_easu_pipeline_layout,
                    vk::ShaderStageFlags::FRAGMENT,
                    0,
                    easu_bytes,
                );
                self.device.cmd_draw(cmd, 3, 1, 0, 0);

                // Run FSR RCAS
                self.device.cmd_bind_pipeline(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.fsr_rcas_pipeline,
                );
                let rcas_pc = FsrRcasConstants {
                    dst_size: [
                        self.swapchain_extent.width as f32,
                        self.swapchain_extent.height as f32,
                    ],
                    sharpness: 0.85,
                    _pad: 0.0,
                };
                let rcas_bytes = std::slice::from_raw_parts(
                    &rcas_pc as *const _ as *const u8,
                    std::mem::size_of::<FsrRcasConstants>(),
                );
                self.device.cmd_push_constants(
                    cmd,
                    self.fsr_rcas_pipeline_layout,
                    vk::ShaderStageFlags::FRAGMENT,
                    0,
                    rcas_bytes,
                );
                self.device.cmd_draw(cmd, 3, 1, 0, 0);
            }

            // 2. Draw 2D UI Overlay (at native display resolution)
            if ui_count > 0 {
                let full_viewport = [vk::Viewport::default()
                    .x(0.0)
                    .y(0.0)
                    .width(self.swapchain_extent.width as f32)
                    .height(self.swapchain_extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0)];
                self.device.cmd_set_viewport(cmd, 0, &full_viewport);

                let full_scissor = [vk::Rect2D::default()
                    .offset(vk::Offset2D { x: 0, y: 0 })
                    .extent(self.swapchain_extent)];
                self.device.cmd_set_scissor(cmd, 0, &full_scissor);

                self.device
                    .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.ui_pipeline);
                self.device.cmd_bind_descriptor_sets(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.ui_pipeline_layout,
                    0,
                    &[self.descriptor_set],
                    &[],
                );
                self.device.cmd_bind_vertex_buffers(
                    cmd,
                    0,
                    &[self.ui_vertex_buffer.buffer],
                    &[0],
                );

                let ui_pc = UiPushConstants {
                    screen_size: [
                        self.swapchain_extent.width as f32,
                        self.swapchain_extent.height as f32,
                    ],
                    use_texture: 0.0,
                    _pad: 0.0,
                };

                let ui_pc_bytes = std::slice::from_raw_parts(
                    &ui_pc as *const _ as *const u8,
                    std::mem::size_of::<UiPushConstants>(),
                );
                self.device.cmd_push_constants(
                    cmd,
                    self.ui_pipeline_layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    ui_pc_bytes,
                );

                self.device.cmd_draw(cmd, ui_count as u32, 1, 0, 0);
            }

            self.device.cmd_end_render_pass(cmd);
            self.device.end_command_buffer(cmd).unwrap();
        }

        // Submit
        let wait_semaphores = [self.image_available_semaphore];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphore];
        let command_buffers = [cmd];

        let submit_info = [vk::SubmitInfo::default()
            .wait_semaphores(if self.is_headless { &[] } else { &wait_semaphores })
            .wait_dst_stage_mask(if self.is_headless { &[] } else { &wait_stages })
            .command_buffers(&command_buffers)
            .signal_semaphores(if self.is_headless { &[] } else { &signal_semaphores })];

        unsafe {
            self.device
                .queue_submit(self.queue, &submit_info, self.in_flight_fence)
                .map_err(|e| format!("Queue submit failed: {:?}", e))?;
        }
        self.fog_image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;

        // Present
        if let (Some(sw_loader), Some(sw)) = (&self.swapchain_loader, self.swapchain) {
            let swapchains = [sw];
            let image_indices = [image_index as u32];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            unsafe {
                sw_loader
                    .queue_present(self.queue, &present_info)
                    .map_err(|e| format!("Queue present failed: {:?}", e))?;
            }
        }

        Ok(())
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            self.device.destroy_fence(self.in_flight_fence, None);
            self.device
                .destroy_semaphore(self.render_finished_semaphore, None);
            self.device
                .destroy_semaphore(self.image_available_semaphore, None);

            self.device.destroy_pipeline(self.fsr_rcas_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.fsr_rcas_pipeline_layout, None);
            self.device.destroy_pipeline(self.fsr_easu_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.fsr_easu_pipeline_layout, None);

            self.device.destroy_pipeline(self.ui_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.ui_pipeline_layout, None);
            self.device.destroy_pipeline(self.world_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.world_pipeline_layout, None);

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            Self::destroy_vulkan_texture(&self.device, &mut self.fog_texture);
            Self::destroy_vulkan_buffer(&self.device, &mut self.fog_staging_buffer);
            Self::destroy_vulkan_buffer(&self.device, &mut self.rt_scene_buffer);

            Self::destroy_vulkan_buffer(&self.device, &mut self.mesh_vertex_buffer);
            Self::destroy_vulkan_buffer(&self.device, &mut self.mesh_index_buffer);
            Self::destroy_vulkan_buffer(&self.device, &mut self.ui_vertex_buffer);

            for &fb in &self.framebuffers {
                self.device.destroy_framebuffer(fb, None);
            }

            self.device.destroy_render_pass(self.render_pass, None);
            self.device.destroy_image_view(self.depth_view, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_memory, None);

            for &view in &self.swapchain_image_views {
                self.device.destroy_image_view(view, None);
            }

            if let (Some(sw_loader), Some(sw)) = (&self.swapchain_loader, self.swapchain) {
                sw_loader.destroy_swapchain(sw, None);
            }

            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);

            if let (Some(surf_loader), Some(surf)) = (&self.surface_loader, self.surface) {
                surf_loader.destroy_surface(surf, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}

impl VulkanRenderer {
    unsafe fn destroy_vulkan_buffer(device: &ash::Device, buf: &mut VulkanBuffer) {
        if buf.buffer != vk::Buffer::null() {
            device.destroy_buffer(buf.buffer, None);
            device.free_memory(buf.memory, None);
            buf.buffer = vk::Buffer::null();
        }
    }

    unsafe fn destroy_vulkan_texture(device: &ash::Device, tex: &mut VulkanTexture) {
        if tex.image != vk::Image::null() {
            device.destroy_sampler(tex.sampler, None);
            device.destroy_image_view(tex.view, None);
            device.destroy_image(tex.image, None);
            device.free_memory(tex.memory, None);
            tex.image = vk::Image::null();
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_memory_type(
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Option<u32> {
    for i in 0..mem_props.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && (mem_props.memory_types[i as usize].property_flags & properties) == properties
        {
            return Some(i);
        }
    }
    None
}

fn create_vulkan_buffer(
    device: &ash::Device,
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<VulkanBuffer, String> {
    let buffer_ci = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = unsafe {
        device
            .create_buffer(&buffer_ci, None)
            .map_err(|e| format!("Buffer create failed: {:?}", e))?
    };

    let mem_req = unsafe { device.get_buffer_memory_requirements(buffer) };
    let mem_type = find_memory_type(mem_props, mem_req.memory_type_bits, properties)
        .ok_or("Failed to find suitable memory type for buffer")?;

    let alloc_ci = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_req.size)
        .memory_type_index(mem_type);

    let memory = unsafe {
        device
            .allocate_memory(&alloc_ci, None)
            .map_err(|e| format!("Buffer memory alloc failed: {:?}", e))?
    };

    unsafe {
        device
            .bind_buffer_memory(buffer, memory, 0)
            .map_err(|e| format!("Buffer memory bind failed: {:?}", e))?;
    }

    Ok(VulkanBuffer {
        buffer,
        memory,
        size,
    })
}

fn create_vulkan_texture(
    device: &ash::Device,
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    width: u32,
    height: u32,
    format: vk::Format,
) -> Result<VulkanTexture, String> {
    let img_ci = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);

    let image = unsafe {
        device
            .create_image(&img_ci, None)
            .map_err(|e| format!("Texture image creation failed: {:?}", e))?
    };

    let mem_req = unsafe { device.get_image_memory_requirements(image) };
    let mem_type = find_memory_type(
        mem_props,
        mem_req.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
    .ok_or("Failed to find memory type for texture image")?;

    let alloc_ci = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_req.size)
        .memory_type_index(mem_type);

    let memory = unsafe {
        device
            .allocate_memory(&alloc_ci, None)
            .map_err(|e| format!("Texture memory alloc failed: {:?}", e))?
    };

    unsafe {
        device
            .bind_image_memory(image, memory, 0)
            .map_err(|e| format!("Texture memory bind failed: {:?}", e))?;
    }

    let view_ci = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1),
        );

    let view = unsafe {
        device
            .create_image_view(&view_ci, None)
            .map_err(|e| format!("Texture view creation failed: {:?}", e))?
    };

    let sampler_ci = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .anisotropy_enable(false)
        .unnormalized_coordinates(false);

    let sampler = unsafe {
        device
            .create_sampler(&sampler_ci, None)
            .map_err(|e| format!("Texture sampler creation failed: {:?}", e))?
    };

    Ok(VulkanTexture {
        image,
        memory,
        view,
        sampler,
        width,
        height,
    })
}

fn create_shader_module(device: &ash::Device, code: &[u32]) -> Result<vk::ShaderModule, String> {
    let ci = vk::ShaderModuleCreateInfo::default().code(code);
    unsafe {
        device
            .create_shader_module(&ci, None)
            .map_err(|e| format!("Failed to create shader module: {:?}", e))
    }
}

fn create_world_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    shader_module: vk::ShaderModule,
    extent: vk::Extent2D,
) -> Result<vk::Pipeline, String> {
    let main_name = std::ffi::CString::new("vs_main").unwrap();
    let frag_name = std::ffi::CString::new("fs_main").unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(shader_module)
            .name(&main_name),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(shader_module)
            .name(&frag_name),
    ];

    let vertex_bindings = [vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(std::mem::size_of::<Vertex3D>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX)];

    let vertex_attributes = [
        vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0),
        vk::VertexInputAttributeDescription::default()
            .location(1)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(12),
        vk::VertexInputAttributeDescription::default()
            .location(2)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(24),
    ];

    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&vertex_bindings)
        .vertex_attribute_descriptions(&vertex_attributes);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewports = [vk::Viewport::default()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0)];

    let scissors = [vk::Rect2D::default()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent)];

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewports(&viewports)
        .scissors(&scissors);

    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false);

    let color_blend_attachment = [vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(false)];

    let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(&color_blend_attachment);

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let pipeline_ci = [vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisampling)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0)];

    let pipeline = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_ci, None)
            .map_err(|(_, e)| format!("World pipeline creation failed: {:?}", e))?[0]
    };

    Ok(pipeline)
}

fn create_ui_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    shader_module: vk::ShaderModule,
    extent: vk::Extent2D,
) -> Result<vk::Pipeline, String> {
    let main_name = std::ffi::CString::new("vs_main").unwrap();
    let frag_name = std::ffi::CString::new("fs_main").unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(shader_module)
            .name(&main_name),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(shader_module)
            .name(&frag_name),
    ];

    let vertex_bindings = [vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(std::mem::size_of::<Vertex2D>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX)];

    let vertex_attributes = [
        vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0),
        vk::VertexInputAttributeDescription::default()
            .location(1)
            .binding(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(8),
        vk::VertexInputAttributeDescription::default()
            .location(2)
            .binding(0)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(16),
    ];

    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&vertex_bindings)
        .vertex_attribute_descriptions(&vertex_attributes);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewports = [vk::Viewport::default()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0)];

    let scissors = [vk::Rect2D::default()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent)];

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewports(&viewports)
        .scissors(&scissors);

    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false);

    let color_blend_attachment = [vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)];

    let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(&color_blend_attachment);

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let pipeline_ci = [vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisampling)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0)];

    let pipeline = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_ci, None)
            .map_err(|(_, e)| format!("UI pipeline creation failed: {:?}", e))?[0]
    };

    Ok(pipeline)
}

fn create_fullscreen_post_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    shader_module: vk::ShaderModule,
    extent: vk::Extent2D,
) -> Result<vk::Pipeline, String> {
    let main_name = std::ffi::CString::new("vs_main").unwrap();
    let frag_name = std::ffi::CString::new("fs_main").unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(shader_module)
            .name(&main_name),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(shader_module)
            .name(&frag_name),
    ];

    // No vertex buffer needed; vertices are generated in vertex shader
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewports = [vk::Viewport::default()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0)];

    let scissors = [vk::Rect2D::default()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent)];

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewports(&viewports)
        .scissors(&scissors);

    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false);

    // Blends or overwrites
    let color_blend_attachment = [vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(false)];

    let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(&color_blend_attachment);

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let pipeline_ci = [vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisampling)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0)];

    let pipeline = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_ci, None)
            .map_err(|(_, e)| format!("FSR post pipeline creation failed: {:?}", e))?[0]
    };

    Ok(pipeline)
}
