use std::{error::Error, sync::Arc};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowAttributes},
};

use crate::{config::AppConfig, renderer::VulkanApp};

struct ExampleApp {
    app: Option<VulkanApp>,
    window: Option<Arc<Window>>,
    show_debug_menu: bool,
    config: AppConfig,
}

impl ApplicationHandler for ExampleApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if self.window.is_some() {
            return;
        }

        let attributes: WindowAttributes = Window::default_attributes()
            .with_title(&self.config.window.title)
            .with_inner_size(PhysicalSize::new(
                self.config.window.size[0],
                self.config.window.size[1],
            ))
            .with_resizable(self.config.window.resizable);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("Failed to create window: {error}");
                event_loop.exit();
                return;
            }
        };
        self.config.scene.on_start();
        let app = match VulkanApp::new(&window, self.config.scene.config(), &self.config.shaders) {
            Ok(app) => app,
            Err(error) => {
                eprintln!("Failed to initialize scene: {error}");
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
            app.handle_window_event(window, window_id, &event, self.show_debug_menu);
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && event.physical_key == PhysicalKey::Code(self.config.debug_toggle_key) =>
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
                    .draw_frame(window, self.show_debug_menu, &mut self.config.scene);
                if let Err(error) = result {
                    eprintln!("Failed to draw scene: {error}");
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

pub fn run(config: AppConfig) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut ExampleApp {
        app: None,
        window: None,
        show_debug_menu: config.show_debug_menu_on_start,
        config,
    })?;
    Ok(())
}
