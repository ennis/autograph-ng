//! Boilerplate code for creating a window and an OpenGL context with winit/glutin.
use std::cell::RefCell;

use config;
use pretty_env_logger;
use winit;

// re-export window event handling stuff.
pub use winit::EventsLoop;
pub use winit::Window;
pub use winit::WindowBuilder;
pub use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    AxisId, ButtonId, DeviceId, ElementState, Event, KeyboardInput, ModifiersState, MouseButton,
    MouseScrollDelta, Touch, TouchPhase, VirtualKeyCode, WindowEvent,
};

use crate::renderer::backend::gl::create_backend_and_window;
use crate::renderer::backend::gl::OpenGlBackend;
use crate::renderer::*;

pub struct App {
    pub cfg: config::Config,
    pub events_loop: RefCell<winit::EventsLoop>,
    pub renderer: Renderer<OpenGlBackend>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> App {
        pretty_env_logger::init();
        let mut cfg = config::Config::default();
        cfg.merge(config::File::with_name("Settings")).unwrap();
        load_environment_config(&mut cfg);

        let events_loop = create_events_loop();
        let window_width = cfg.get::<u32>("gfx.window.width").unwrap();
        let window_height = cfg.get::<u32>("gfx.window.height").unwrap();
        let fullscreen = cfg.get::<u32>("gfx.window.fullscreen").unwrap();
        let vsync = cfg.get::<bool>("gfx.window.vsync").unwrap();
        let window_title = cfg.get::<String>("gfx.window.title").unwrap();
        let window_builder = winit::WindowBuilder::new()
            .with_title(window_title.clone())
            .with_dimensions((window_width, window_height).into());

        let backend = create_backend_and_window(&cfg, &events_loop, window_builder);
        let renderer = Renderer::new(backend);

        App {
            events_loop: RefCell::new(events_loop),
            cfg,
            renderer,
        }
    }

    pub fn poll_events<F>(&self, mut callback: F) -> bool
    where
        F: FnMut(winit::Event),
    {
        let mut should_close = false;
        self.events_loop.borrow_mut().poll_events(|event| {
            // event handling
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    should_close = true;
                }
                // if resize, then delete persistent resources and re-create
                _ => callback(event),
            }
        });
        should_close
    }

    pub fn renderer(&self) -> &Renderer<OpenGlBackend> {
        &self.renderer
    }
}

pub fn load_environment_config(cfg: &mut config::Config) {
    cfg.merge(config::Environment::with_prefix("GFX")).unwrap();
}

pub fn create_events_loop() -> EventsLoop {
    winit::EventsLoop::new()
}
