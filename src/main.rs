use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attributes = Window::default_attributes()
                .with_title("Nishant's Lab")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));

            match event_loop.create_window(attributes) {
                Ok(window) => {
                    self.window = Some(window);
                    println!("Window created successfully!");
                }
                Err(error) => {
                    eprintln!("Failed to create window: {error}");
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Closing Nishant's Lab");
                event_loop.exit();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::default();

    event_loop
        .run_app(&mut app)
        .expect("Failed to run application");
}
