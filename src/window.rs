//! Boilerplate code for creating a window and an OpenGL context with winit/glutin.

use std::ffi::{CString, CStr};
use std::ptr;
use std::os::raw::{c_char, c_void};
use std::mem;
use std::u32;
use std::rc::Rc;

use winit;
use config;
use ash;
use ash::extensions;
use ash::vk;
use pretty_env_logger;

use context::{Context, PresentationTarget, Presentation};

// re-export window event handling stuff.
pub use winit::EventsLoop;
pub use winit::WindowBuilder;
pub use winit::Window;
pub use winit::{Event,
                 WindowEvent,
                 MouseButton,
                 MouseScrollDelta,
                 KeyboardInput,
                 VirtualKeyCode,
                 ElementState,
                 ModifiersState,
                 DeviceId,
                 AxisId,
                 Touch,
                 ButtonId,
                 TouchPhase,
                 dpi::{LogicalPosition,
                       LogicalSize,
                       PhysicalPosition,
                       PhysicalSize}};

pub struct App
{
    pub cfg: config::Config,
    pub events_loop: winit::EventsLoop,
    pub window: Rc<winit::Window>,
    pub context: Context,
    pub presentation: Presentation
}

impl App
{
    pub fn new() -> App {
        pretty_env_logger::init();
        let mut cfg = config::Config::default();
        cfg.merge(config::File::with_name("Settings")).unwrap();
        load_environment_config(&mut cfg);

        let mut events_loop = create_events_loop();
        let (mut window, mut context, mut presentation) = create_window_and_context(&events_loop, &cfg);

        App {
            cfg,
            events_loop,
            window,
            context,
            presentation
        }
    }
}

pub fn load_environment_config(cfg: &mut config::Config)
{
    cfg.merge(config::Environment::with_prefix("GFX")).unwrap();
}

pub fn create_events_loop() -> EventsLoop
{
    winit::EventsLoop::new()
}

pub fn create_window_and_context(events_loop: &EventsLoop, cfg: &config::Config) -> (Rc<Window>, Context, Presentation)
{
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
    let (context, presentations) = Context::new(&[&presentation_target], cfg);

    (window, context, presentations.first().unwrap().clone())
}

// create instance
// create window
// create surface
// bundle window+surface together
//
// create context (optionally with window) -> instance + device
// create presentation target (from context + window + surface)
//
// Alt:
// create context (phase 1, without device) -> instance
// create presentation target (context + window) -> device + surface + swapchain, bundle window, surface and swapchain
// -> deferred physical device selection
//
// Device creation: must handle all potential surface types
// -> must know surface types in advance (actually, create the surfaces before)
//
// create window + surface pair
//
// V3:
// - create Context (entry+instance)
// - create Window (by user)
// - Context + Window -> PresentationWindow(Window + Surface)
// - create Renderer: Context + PresentationWindow -> Renderer(Context,
// - create swapchains: Renderer + PresentationWindow -> PresentationTarget

// idea: presentation targets are dumb objects
// they contain data that can be deleted, but useless since they have no operations of their own.

/*fn main()
{
    // presentation target is lifetime-bound to context
    //let (window, mut presentation_target) = create_window_and_presentation_target(&cfg);        // bundle window + vk surface

    let target = PresentationTarget::main_screen();

    // PresentationTarget: Cell<Option<Rc<PresentationTargetInner>>>

    let context = Context::new(&mut [&mut target], &cfg); // create a compatible renderer for the presentation targets

    // can create new presentation targets here
    let other_target = PresentationTarget::window(&window);
}*/