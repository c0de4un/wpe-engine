use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

pub trait Game {
    fn on_create(&mut self, _window: &Window) {}
    fn on_update(&mut self, _dt: f32) {}
    fn on_render(&mut self) {}
}

struct EngineApp<G: Game> {
    game: G,
    window: Option<Window>,
}

impl<G: Game + 'static> ApplicationHandler for EngineApp<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = WindowAttributes::default()
                .with_title("Wingman Protocol EX")
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

            let window = event_loop.create_window(attrs).unwrap();
            self.game.on_create(&window);
            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Engine: Close requested. Shutting down.");
                event_loop.exit();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let dt = 0.016;

        self.game.on_update(dt);
        self.game.on_render();

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

pub fn run<G: Game + 'static>(mut game: G) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = EngineApp {
        game,
        window: None,
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}