//! Boilerplate code for creating a window and an OpenGL context with winit/glutin.
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::Arc;
use std::u32;

use ash;
use ash::extensions;
use ash::vk;
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

use crate::device::Device;
use crate::instance::Instance;
use crate::surface::Surface;
use crate::swapchain::Swapchain;

pub struct App {
    pub cfg: config::Config,
    pub events_loop: RefCell<winit::EventsLoop>,
    pub window: Arc<winit::Window>,
    pub instance: Arc<Instance>,
    pub device: Arc<Device>,
    pub surface: Arc<Surface>,
    pub swapchain: Arc<Swapchain>,
}

impl App {
    pub fn new() -> App {
        pretty_env_logger::init();
        let mut cfg = config::Config::default();
        cfg.merge(config::File::with_name("Settings")).unwrap();
        load_environment_config(&mut cfg);

        let mut events_loop = create_events_loop();
        let mut window = create_window(&events_loop, &cfg);

        let instance = Instance::new(&cfg);
        let surface = Surface::from_window(&instance, &window, &cfg);
        let device = Device::new(&instance, &cfg, Some(&surface));

        let dimensions: (u32, u32) = window.get_inner_size().unwrap().into();

        let swapchain = Swapchain::new(
            &device,
            &surface,
            device.max_frames_in_flight(),
            vk::Format::B8g8r8a8Srgb,
            vk::ColorSpaceKHR::SrgbNonlinear,
            dimensions,
            1,
            vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT | vk::IMAGE_USAGE_STORAGE_BIT,
            vk::SURFACE_TRANSFORM_IDENTITY_BIT_KHR,
            vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
            None,
        )
        .expect("unable to create swapchain");

        App {
            cfg,
            events_loop: RefCell::new(events_loop),
            window,
            instance,
            device,
            surface,
            swapchain,
        }
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub fn window(&self) -> &Arc<Window> {
        &self.window
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

    pub fn swapchain(&self) -> &Arc<Swapchain> {
        &self.swapchain
    }

    /*pub fn next_swapchain_image<'a>(&'a self) -> SwapchainImage<'a> {
        self.swapchain.next_image()
    }*/
}

pub fn load_environment_config(cfg: &mut config::Config) {
    cfg.merge(config::Environment::with_prefix("GFX")).unwrap();
}

pub fn create_events_loop() -> EventsLoop {
    winit::EventsLoop::new()
}

pub fn create_window(events_loop: &EventsLoop, cfg: &config::Config) -> Arc<Window> {
    let window_width = cfg.get::<u32>("gfx.window.width").unwrap();
    let window_height = cfg.get::<u32>("gfx.window.height").unwrap();
    let fullscreen = cfg.get::<u32>("gfx.window.fullscreen").unwrap();
    let vsync = cfg.get::<bool>("gfx.window.vsync").unwrap();
    let window_title = cfg.get::<String>("gfx.window.title").unwrap();

    // create a window
    let window_builder = winit::WindowBuilder::new()
        .with_title(window_title.clone())
        .with_dimensions((window_width, window_height).into());
    let window = Arc::new(window_builder.build(events_loop).unwrap());

    window
}
