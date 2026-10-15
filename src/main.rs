use std::{error::Error, ffi::CString, io::Cursor, mem::size_of, ptr, sync::Arc, time::Instant};

use ash::{Entry, vk};
use imgui::{Context, DrawData};
use imgui_rs_vulkan_renderer::{Options as ImGuiRendererOptions, Renderer as ImGuiRenderer};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

fn parse_obj(source: &str) -> Result<Vec<Vertex>, Box<dyn Error>> {
    let mut positions = Vec::new();
    let mut vertices = Vec::new();
    let mut color = [0.8, 0.8, 0.8];

    for (line_number, line) in source.lines().enumerate() {
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("v") => {
                let coordinates = fields
                    .take(3)
                    .map(str::parse::<f32>)
                    .collect::<Result<Vec<_>, _>>()?;
                if coordinates.len() != 3 {
                    return Err(format!(
                        "OBJ line {}: expected 3 vertex coordinates",
                        line_number + 1
                    )
                    .into());
                }
                positions.push([coordinates[0], coordinates[1], coordinates[2]]);
            }
            Some("g") | Some("usemtl") => {
                color = match fields.next().unwrap_or_default() {
                    "red" => [0.95, 0.12, 0.12],
                    "blue" => [0.12, 0.32, 0.95],
                    "yellow" => [0.98, 0.78, 0.08],
                    "green" => [0.08, 0.72, 0.24],
                    _ => [0.8, 0.8, 0.8],
                };
            }
            Some("f") => {
                let indices = fields
                    .map(|field| {
                        let raw_index = field
                            .split('/')
                            .next()
                            .unwrap_or_default()
                            .parse::<isize>()?;
                        let index = if raw_index > 0 {
                            raw_index - 1
                        } else {
                            positions.len() as isize + raw_index
                        };
                        usize::try_from(index).map_err(|_| {
                            format!("OBJ line {}: invalid vertex index", line_number + 1).into()
                        })
                    })
                    .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
                if indices.len() < 3 {
                    return Err(format!(
                        "OBJ line {}: face needs at least 3 vertices",
                        line_number + 1
                    )
                    .into());
                }
                for index in &indices {
                    if *index >= positions.len() {
                        return Err(format!(
                            "OBJ line {}: vertex index is out of range",
                            line_number + 1
                        )
                        .into());
                    }
                }
                for corner in 1..indices.len() - 1 {
                    for index in [indices[0], indices[corner], indices[corner + 1]] {
                        vertices.push(Vertex {
                            position: positions[index],
                            color,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if vertices.is_empty() {
        return Err("OBJ model contains no faces".into());
    }
    Ok(vertices)
}

struct SwapchainResources {
    loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,
    extent: vk::Extent2D,
    depth_image: vk::Image,
    depth_memory: vk::DeviceMemory,
    depth_view: vk::ImageView,
}

impl SwapchainResources {
    unsafe fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            for framebuffer in self.framebuffers.drain(..) {
                device.destroy_framebuffer(framebuffer, None);
            }
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_render_pass(self.render_pass, None);
            for image_view in self.image_views.drain(..) {
                device.destroy_image_view(image_view, None);
            }
            device.destroy_image_view(self.depth_view, None);
            device.destroy_image(self.depth_image, None);
            device.free_memory(self.depth_memory, None);
            self.loader.destroy_swapchain(self.swapchain, None);
        }
    }
}

struct VulkanApp {
    _entry: Entry,
    instance: ash::Instance,
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    device: ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
    resources: SwapchainResources,
    vertex_buffer: vk::Buffer,
    vertex_memory: vk::DeviceMemory,
    vertex_count: u32,
    imgui: Context,
    imgui_platform: WinitPlatform,
    imgui_renderer: Option<ImGuiRenderer>,
    started_at: Instant,
    last_frame_at: Instant,
}

impl VulkanApp {
    fn new(window: &Window) -> Result<Self, Box<dyn Error>> {
        let entry = unsafe { Entry::load()? };
        let app_name = CString::new("Rotating Pyraminx")?;
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&app_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_0);

        let display_handle = window.display_handle()?.as_raw();
        let window_handle = window.window_handle()?.as_raw();
        let extensions = ash_window::enumerate_required_extensions(display_handle)?;
        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(extensions);
        let instance = unsafe { entry.create_instance(&instance_info, None)? };
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)?
        };
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        let (physical_device, queue_family) =
            unsafe { Self::pick_physical_device(&instance, &surface_loader, surface)? };
        let priorities = [1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities)];
        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];
        let device_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extensions);
        let device = unsafe { instance.create_device(physical_device, &device_info, None)? };
        let queue = unsafe { device.get_device_queue(queue_family, 0) };
        let mesh = parse_obj(include_str!("models/pyraminx.obj"))?;
        let (vertex_buffer, vertex_memory) =
            unsafe { Self::create_vertex_buffer(&instance, physical_device, &device, &mesh)? };

        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.create_command_pool(&command_pool_info, None)? };
        let resources = unsafe {
            Self::create_swapchain_resources(
                &instance,
                physical_device,
                &device,
                &surface_loader,
                surface,
                queue_family,
                window,
            )?
        };
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(resources.framebuffers.len() as u32);
        let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info)? };
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let image_available = unsafe { device.create_semaphore(&semaphore_info, None)? };
        let render_finished = unsafe { device.create_semaphore(&semaphore_info, None)? };
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let in_flight = unsafe { device.create_fence(&fence_info, None)? };

        let mut imgui = Context::create();
        imgui.set_ini_filename(None);
        let mut imgui_platform = WinitPlatform::new(&mut imgui);
        imgui_platform.attach_window(imgui.io_mut(), window, HiDpiMode::Default);
        let imgui_renderer = ImGuiRenderer::with_default_allocator(
            &instance,
            physical_device,
            device.clone(),
            queue,
            command_pool,
            resources.render_pass,
            &mut imgui,
            Some(ImGuiRendererOptions {
                in_flight_frames: 1,
                ..Default::default()
            }),
        )?;

        let started_at = Instant::now();
        Ok(Self {
            _entry: entry,
            instance,
            surface_loader,
            surface,
            device,
            queue,
            command_pool,
            command_buffers,
            image_available,
            render_finished,
            in_flight,
            resources,
            vertex_buffer,
            vertex_memory,
            vertex_count: mesh.len() as u32,
            imgui,
            imgui_platform,
            imgui_renderer: Some(imgui_renderer),
            started_at,
            last_frame_at: started_at,
        })
    }

    unsafe fn create_vertex_buffer(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        vertices: &[Vertex],
    ) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn Error>> {
        let size = (vertices.len() * size_of::<Vertex>()) as vk::DeviceSize;
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let memory_type = (0..properties.memory_type_count)
            .find(|index| {
                requirements.memory_type_bits & (1 << index) != 0
                    && properties.memory_types[*index as usize]
                        .property_flags
                        .contains(
                            vk::MemoryPropertyFlags::HOST_VISIBLE
                                | vk::MemoryPropertyFlags::HOST_COHERENT,
                        )
            })
            .ok_or("No host-visible coherent memory for OBJ vertex buffer")?;
        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type);
        let memory = unsafe { device.allocate_memory(&allocate_info, None)? };
        unsafe {
            device.bind_buffer_memory(buffer, memory, 0)?;
            let mapped = device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty())?;
            ptr::copy_nonoverlapping(
                vertices.as_ptr().cast::<u8>(),
                mapped.cast::<u8>(),
                size as usize,
            );
            device.unmap_memory(memory);
        }
        Ok((buffer, memory))
    }

    unsafe fn pick_physical_device(
        instance: &ash::Instance,
        surface_loader: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> Result<(vk::PhysicalDevice, u32), Box<dyn Error>> {
        for physical_device in unsafe { instance.enumerate_physical_devices()? } {
            let queue_families =
                unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
            for (index, family) in queue_families.iter().enumerate() {
                let present_support = unsafe {
                    surface_loader.get_physical_device_surface_support(
                        physical_device,
                        index as u32,
                        surface,
                    )?
                };
                if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && present_support {
                    let extensions =
                        unsafe { instance.enumerate_device_extension_properties(physical_device)? };
                    let has_swapchain = extensions.iter().any(|extension| unsafe {
                        std::ffi::CStr::from_ptr(extension.extension_name.as_ptr())
                            == ash::khr::swapchain::NAME
                    });
                    let formats = unsafe {
                        surface_loader
                            .get_physical_device_surface_formats(physical_device, surface)?
                    };
                    let modes = unsafe {
                        surface_loader
                            .get_physical_device_surface_present_modes(physical_device, surface)?
                    };
                    if has_swapchain && !formats.is_empty() && !modes.is_empty() {
                        return Ok((physical_device, index as u32));
                    }
                }
            }
        }
        Err("No Vulkan device supports graphics and presentation for this window".into())
    }

    unsafe fn create_swapchain_resources(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        surface_loader: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
        queue_family: u32,
        window: &Window,
    ) -> Result<SwapchainResources, Box<dyn Error>> {
        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(physical_device, surface)?
        };
        let surface_format = formats
            .iter()
            .copied()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(formats[0]);
        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
        };
        let window_size = window.inner_size();
        let extent = if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: window_size.width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: window_size.height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            }
        };
        let image_count =
            (capabilities.min_image_count + 1).min(if capabilities.max_image_count == 0 {
                u32::MAX
            } else {
                capabilities.max_image_count
            });
        let queue_family_indices = [queue_family];
        let composite_alpha = [
            vk::CompositeAlphaFlagsKHR::OPAQUE,
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
            vk::CompositeAlphaFlagsKHR::INHERIT,
        ]
        .into_iter()
        .find(|alpha| capabilities.supported_composite_alpha.contains(*alpha))
        .ok_or("No supported composite alpha mode")?;
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(composite_alpha)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true);
        let loader = ash::khr::swapchain::Device::new(instance, device);
        let swapchain = unsafe { loader.create_swapchain(&create_info, None)? };
        let images = unsafe { loader.get_swapchain_images(swapchain)? };
        let mut image_views = Vec::with_capacity(images.len());
        for image in &images {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(vk::ComponentMapping::default())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            image_views.push(unsafe { device.create_image_view(&view_info, None)? });
        }

        let depth_format = [
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ]
        .into_iter()
        .find(|format| {
            let properties =
                unsafe { instance.get_physical_device_format_properties(physical_device, *format) };
            properties
                .optimal_tiling_features
                .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
        })
        .ok_or("No supported depth-buffer format")?;
        let depth_image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(depth_format)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let depth_image = unsafe { device.create_image(&depth_image_info, None)? };
        let depth_requirements = unsafe { device.get_image_memory_requirements(depth_image) };
        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let depth_memory_type = (0..memory_properties.memory_type_count)
            .find(|index| {
                depth_requirements.memory_type_bits & (1 << index) != 0
                    && memory_properties.memory_types[*index as usize]
                        .property_flags
                        .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            })
            .ok_or("No device-local memory for depth buffer")?;
        let depth_memory = unsafe {
            device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(depth_requirements.size)
                    .memory_type_index(depth_memory_type),
                None,
            )?
        };
        unsafe { device.bind_image_memory(depth_image, depth_memory, 0)? };
        let depth_aspect = if matches!(
            depth_format,
            vk::Format::D32_SFLOAT_S8_UINT | vk::Format::D24_UNORM_S8_UINT
        ) {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        } else {
            vk::ImageAspectFlags::DEPTH
        };
        let depth_view = unsafe {
            device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(depth_image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(depth_format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: depth_aspect,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    }),
                None,
            )?
        };

        let attachments = [
            vk::AttachmentDescription::default()
                .format(surface_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR),
            vk::AttachmentDescription::default()
                .format(depth_format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
        ];
        let color_attachment = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
        let subpasses = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment)
            .depth_stencil_attachment(&depth_attachment)];
        let dependencies = [vk::SubpassDependency::default()
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
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            )];
        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        let render_pass = unsafe { device.create_render_pass(&render_pass_info, None)? };

        let vertex_code = ash::util::read_spv(&mut Cursor::new(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/pyraminx.vert.spv"
        ))))?;
        let fragment_code = ash::util::read_spv(&mut Cursor::new(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/pyraminx.frag.spv"
        ))))?;
        let vertex_module = unsafe {
            device.create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(&vertex_code),
                None,
            )?
        };
        let fragment_module = unsafe {
            device.create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(&fragment_code),
                None,
            )?
        };
        let entry_name = CString::new("main")?;
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_module)
                .name(&entry_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_module)
                .name(&entry_name),
        ];
        let vertex_bindings = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_attributes = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: size_of::<[f32; 3]>() as u32,
            },
        ];
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&vertex_bindings)
            .vertex_attribute_descriptions(&vertex_attributes);
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);
        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
        let push_constant_ranges = [vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: size_of::<f32>() as u32,
        }];
        let pipeline_layout = unsafe {
            device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .push_constant_ranges(&push_constant_ranges),
                None,
            )?
        };
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
        let pipeline_info = [vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0)];
        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_info, None)
                .map_err(|(_, error)| error)?[0]
        };
        unsafe {
            device.destroy_shader_module(vertex_module, None);
            device.destroy_shader_module(fragment_module, None);
        }

        let mut framebuffers = Vec::with_capacity(image_views.len());
        for image_view in &image_views {
            let attachments = [*image_view, depth_view];
            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            framebuffers.push(unsafe { device.create_framebuffer(&framebuffer_info, None)? });
        }

        Ok(SwapchainResources {
            loader,
            swapchain,
            image_views,
            render_pass,
            pipeline_layout,
            pipeline,
            framebuffers,
            extent,
            depth_image,
            depth_memory,
            depth_view,
        })
    }

    fn draw_frame(&mut self, window: &Window, show_debug_menu: bool) -> Result<(), Box<dyn Error>> {
        let now = Instant::now();
        self.imgui
            .io_mut()
            .update_delta_time(now.duration_since(self.last_frame_at));
        self.last_frame_at = now;
        self.imgui_platform
            .prepare_frame(self.imgui.io_mut(), window)?;

        let elapsed_seconds = self.started_at.elapsed().as_secs_f32();
        let ui = self.imgui.frame();
        if show_debug_menu {
            ui.window("Debug Menu (F3)")
                .size([300.0, 120.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text(format!("Frame rate: {:.1} FPS", ui.io().framerate));
                    ui.text(format!("Uptime: {elapsed_seconds:.1} seconds"));
                    ui.text(format!("Scene vertices: {}", self.vertex_count));
                });
        }
        self.imgui_platform.prepare_render(ui, window);
        let draw_data = self.imgui.render();

        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight], true, u64::MAX)?;
            let (image_index, _) = self.resources.loader.acquire_next_image(
                self.resources.swapchain,
                u64::MAX,
                self.image_available,
                vk::Fence::null(),
            )?;
            let command_buffer = self.command_buffers[image_index as usize];
            self.device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
            let angle = self.started_at.elapsed().as_secs_f32() * 0.8;
            Self::record_commands(
                &self.device,
                &self.resources,
                self.vertex_buffer,
                self.vertex_count,
                command_buffer,
                image_index,
                angle,
                self.imgui_renderer
                    .as_mut()
                    .expect("ImGui renderer is initialized with the Vulkan app"),
                draw_data,
            )?;
            self.device.reset_fences(&[self.in_flight])?;
            let wait_semaphores = [self.image_available];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [command_buffer];
            let signal_semaphores = [self.render_finished];
            let submit_info = [vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores)];
            self.device
                .queue_submit(self.queue, &submit_info, self.in_flight)?;

            let swapchains = [self.resources.swapchain];
            let image_indices = [image_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);
            self.resources
                .loader
                .queue_present(self.queue, &present_info)?;
        }
        Ok(())
    }

    unsafe fn record_commands(
        device: &ash::Device,
        resources: &SwapchainResources,
        vertex_buffer: vk::Buffer,
        vertex_count: u32,
        command_buffer: vk::CommandBuffer,
        image_index: u32,
        angle: f32,
        imgui_renderer: &mut ImGuiRenderer,
        draw_data: &DrawData,
    ) -> Result<(), Box<dyn Error>> {
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default();
            device.begin_command_buffer(command_buffer, &begin_info)?;
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.03, 0.03, 0.06, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(resources.render_pass)
                .framebuffer(resources.framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: resources.extent,
                })
                .clear_values(&clear_values);
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                resources.pipeline,
            );
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
            device.cmd_push_constants(
                command_buffer,
                resources.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                std::slice::from_raw_parts((&angle as *const f32).cast::<u8>(), size_of::<f32>()),
            );
            device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: resources.extent.width as f32,
                    height: resources.extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            device.cmd_set_scissor(
                command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: resources.extent,
                }],
            );
            device.cmd_draw(command_buffer, vertex_count, 1, 0, 0);
            imgui_renderer.cmd_draw(command_buffer, draw_data)?;
            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer)?;
        }
        Ok(())
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            drop(self.imgui_renderer.take());
            self.device.destroy_fence(self.in_flight, None);
            self.device.destroy_semaphore(self.render_finished, None);
            self.device.destroy_semaphore(self.image_available, None);
            self.device.destroy_command_pool(self.command_pool, None);
            self.resources.destroy(&self.device);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_memory, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

struct ExampleApp {
    app: Option<VulkanApp>,
    window: Option<Arc<Window>>,
    show_debug_menu: bool,
}

impl ApplicationHandler for ExampleApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if self.window.is_some() {
            return;
        }

        let attributes: WindowAttributes = Window::default_attributes()
            .with_title("Rotating Pyraminx")
            .with_inner_size(PhysicalSize::new(800, 600))
            .with_resizable(false);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("Failed to create window: {error}");
                event_loop.exit();
                return;
            }
        };
        let app = match VulkanApp::new(&window) {
            Ok(app) => app,
            Err(error) => {
                eprintln!("Failed to initialize Vulkan: {error}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window);
        self.app = Some(app);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if !self
            .window
            .as_ref()
            .is_some_and(|window| window.id() == window_id)
        {
            return;
        }

        if let (Some(app), Some(window)) = (&mut self.app, &self.window) {
            let platform_event: Event<()> = Event::WindowEvent {
                window_id,
                event: event.clone(),
            };
            app.imgui_platform
                .handle_event(app.imgui.io_mut(), window, &platform_event);
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && event.physical_key == PhysicalKey::Code(KeyCode::F3) =>
            {
                self.show_debug_menu = !self.show_debug_menu;
            }
            WindowEvent::RedrawRequested => {
                let window = self
                    .window
                    .as_ref()
                    .expect("window is initialized with the Vulkan app");
                let result = self
                    .app
                    .as_mut()
                    .expect("Vulkan app is initialized with the window")
                    .draw_frame(window, self.show_debug_menu);
                if let Err(error) = result {
                    eprintln!("Failed to draw Pyraminx: {error}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut ExampleApp {
        app: None,
        window: None,
        show_debug_menu: false,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pyraminx_obj_loads_four_colored_faces() {
        let vertices = parse_obj(include_str!("models/pyraminx.obj")).unwrap();

        assert_eq!(vertices.len(), 12);
        assert_eq!(vertices[0].color, [0.95, 0.12, 0.12]);
        assert_eq!(vertices[3].color, [0.12, 0.32, 0.95]);
        assert_eq!(vertices[6].color, [0.98, 0.78, 0.08]);
        assert_eq!(vertices[9].color, [0.08, 0.72, 0.24]);
    }

    #[test]
    fn obj_parser_triangulates_polygon_faces() {
        let vertices =
            parse_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\ng blue\nf 1 2 3 4\n").unwrap();

        assert_eq!(vertices.len(), 6);
        assert!(
            vertices
                .iter()
                .all(|vertex| vertex.color == [0.12, 0.32, 0.95])
        );
    }
}
