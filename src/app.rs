use std::{error::Error, sync::Arc};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use crate::renderer::VulkanApp;

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
            app.handle_window_event(window, window_id, &event);
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

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut ExampleApp {
        app: None,
        window: None,
        show_debug_menu: false,
    })?;
    Ok(())
}
