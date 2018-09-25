//! Boilerplate code for creating a window and an OpenGL context with winit/glutin.
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::rc::Rc;
use std::u32;

use ash;
use ash::extensions;
use ash::vk;
use config;
use pretty_env_logger;
use winit;

use context::{Context, Presentation, PresentationTarget};

// re-export window event handling stuff.
pub use winit::EventsLoop;
pub use winit::Window;
pub use winit::WindowBuilder;
pub use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    AxisId, ButtonId, DeviceId, ElementState, Event, KeyboardInput, ModifiersState, MouseButton,
    MouseScrollDelta, Touch, TouchPhase, VirtualKeyCode, WindowEvent,
};

pub struct App {
    pub cfg: config::Config,
    pub events_loop: winit::EventsLoop,
    pub window: Rc<winit::Window>,
    pub context: Context,
    pub presentation: Presentation,
}

impl App {
    pub fn new() -> App {
        pretty_env_logger::init();
        let mut cfg = config::Config::default();
        cfg.merge(config::File::with_name("Settings")).unwrap();
        load_environment_config(&mut cfg);

        let mut events_loop = create_events_loop();
        let (mut window, mut context, mut presentation) =
            create_window_and_context(&events_loop, &cfg);

        App {
            cfg,
            events_loop,
            window,
            context,
            presentation,
        }
    }
}

pub fn load_environment_config(cfg: &mut config::Config) {
    cfg.merge(config::Environment::with_prefix("GFX")).unwrap();
}

pub fn create_events_loop() -> EventsLoop {
    winit::EventsLoop::new()
}

pub fn create_window_and_context(
    events_loop: &EventsLoop,
    cfg: &config::Config,
) -> (Rc<Window>, Context, Presentation) {
    let window_width = cfg.get::<u32>("gfx.window.width").unwrap();
    let window_height = cfg.get::<u32>("gfx.window.height").unwrap();
    let fullscreen = cfg.get::<u32>("gfx.window.fullscreen").unwrap();
    let vsync = cfg.get::<bool>("gfx.window.vsync").unwrap();
    let window_title = cfg.get::<String>("gfx.window.title").unwrap();

    // create a window
    let window_builder = winit::WindowBuilder::new()
        .with_title(window_title.clone())
        .with_dimensions((window_width, window_height).into());
    let window = Rc::new(window_builder.build(events_loop).unwrap());

    // presentation context, attached to the window
    let presentation_target = PresentationTarget::Window(window.clone());

    // context
    let (context, mut presentations) = Context::new(&[&presentation_target], cfg);
    let presentation = presentations.drain(..).next().unwrap();

    (window, context, presentation)
}
