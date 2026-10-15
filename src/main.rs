use std::{error::Error, ffi::CString, io::Cursor, sync::Arc};

use ash::{Entry, vk};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
};

struct SwapchainResources {
    loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,
    extent: vk::Extent2D,
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
}

impl VulkanApp {
    fn new(window: &Window) -> Result<Self, Box<dyn Error>> {
        let entry = unsafe { Entry::load()? };
        let app_name = CString::new("Vulkan triangle")?;
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
        })
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

        let attachments = [vk::AttachmentDescription::default()
            .format(surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
        let color_attachment = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let subpasses = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment)];
        let dependencies = [vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];
        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        let render_pass = unsafe { device.create_render_pass(&render_pass_info, None)? };

        let vertex_code = ash::util::read_spv(&mut Cursor::new(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/triangle.vert.spv"
        ))))?;
        let fragment_code = ash::util::read_spv(&mut Cursor::new(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/triangle.frag.spv"
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
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
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
        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)?
        };
        let pipeline_info = [vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization)
            .multisample_state(&multisampling)
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
            let attachments = [*image_view];
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
        })
    }

    fn draw_frame(&self) -> Result<(), Box<dyn Error>> {
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
            self.record_commands(command_buffer, image_index)?;
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
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: u32,
    ) -> Result<(), Box<dyn Error>> {
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default();
            self.device
                .begin_command_buffer(command_buffer, &begin_info)?;
            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.03, 0.03, 0.06, 1.0],
                },
            }];
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.resources.render_pass)
                .framebuffer(self.resources.framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.resources.extent,
                })
                .clear_values(&clear_values);
            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.resources.pipeline,
            );
            self.device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: self.resources.extent.width as f32,
                    height: self.resources.extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            self.device.cmd_set_scissor(
                command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.resources.extent,
                }],
            );
            self.device.cmd_draw(command_buffer, 3, 1, 0, 0);
            self.device.cmd_end_render_pass(command_buffer);
            self.device.end_command_buffer(command_buffer)?;
        }
        Ok(())
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_fence(self.in_flight, None);
            self.device.destroy_semaphore(self.render_finished, None);
            self.device.destroy_semaphore(self.image_available, None);
            self.device.destroy_command_pool(self.command_pool, None);
            self.resources.destroy(&self.device);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

struct ExampleApp {
    app: Option<VulkanApp>,
    window: Option<Arc<Window>>,
    rendered: bool,
}

impl ApplicationHandler for ExampleApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes: WindowAttributes = Window::default_attributes()
            .with_title("Vulkan triangle")
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
        window.request_redraw();
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

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested if !self.rendered => {
                let result = self
                    .app
                    .as_ref()
                    .expect("Vulkan app is initialized with the window")
                    .draw_frame();
                if let Err(error) = result {
                    eprintln!("Failed to draw triangle: {error}");
                    event_loop.exit();
                } else {
                    self.rendered = true;
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut ExampleApp {
        app: None,
        window: None,
        rendered: false,
    })?;
    Ok(())
}
