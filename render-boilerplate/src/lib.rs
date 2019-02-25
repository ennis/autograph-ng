//! Boilerplate code for creating a window and an OpenGL context with winit/glutin.
extern crate image as img;
use self::img::GenericImageView;
use autograph_render::*;
use autograph_render_gl::{create_instance_and_window, OpenGlBackend};
use config;
use pretty_env_logger;
use std::{cell::RefCell, error, fmt, path::Path};
use winit;

use glutin::GlWindow;
use std::sync::Arc;
pub use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    AxisId, ButtonId, DeviceId, ElementState, Event, EventsLoop, KeyboardInput, ModifiersState,
    MouseButton, MouseScrollDelta, Touch, TouchPhase, VirtualKeyCode, Window, WindowBuilder,
    WindowEvent,
};

#[derive(Debug)]
pub enum ImageLoadError {
    UnsupportedColorType(img::ColorType),
    Other(img::ImageError),
}

impl From<img::ImageError> for ImageLoadError {
    fn from(err: img::ImageError) -> Self {
        ImageLoadError::Other(err)
    }
}

impl fmt::Display for ImageLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ImageLoadError::UnsupportedColorType(color_type) => {
                write!(f, "unsupported color type: {:?}", color_type)
            }
            ImageLoadError::Other(err) => err.fmt(f),
        }
    }
}

impl error::Error for ImageLoadError {}

//
pub fn load_image_2d<'a, B: Backend, P: AsRef<Path>>(
    arena: &'a Arena<B>,
    path: P,
) -> Result<Image<'a, B>, ImageLoadError> {
    let img = img::open(path)?;
    let (width, height) = img.dimensions();
    let format = match img.color() {
        img::ColorType::RGB(8) => Format::R8G8B8_SRGB,
        img::ColorType::RGBA(8) => Format::R8G8B8A8_SRGB,
        other => return Err(ImageLoadError::UnsupportedColorType(other)),
    };
    let bytes: &[u8] = match img {
        img::DynamicImage::ImageRgb8(ref rgb) => &*rgb,
        img::DynamicImage::ImageRgba8(ref rgba) => &*rgba,
        _ => panic!(""),
    };

    Ok(arena.create_immutable_image(
        format,
        (width, height).into(),
        MipmapsCount::One,
        1,
        ImageUsageFlags::SAMPLED,
        bytes,
    ))
}

pub struct App {
    pub cfg: config::Config,
    pub events_loop: RefCell<winit::EventsLoop>,
    pub renderer: Renderer<OpenGlBackend>,
    pub window: Arc<GlWindow>,
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
        // cfg.merge(config::File::with_name("Settings")).unwrap();
        load_environment_config(&mut cfg);

        let events_loop = create_events_loop();
        let window_width = cfg.get::<u32>("gfx.window.width").unwrap_or(640);
        let window_height = cfg.get::<u32>("gfx.window.height").unwrap_or(480);
        let _fullscreen = cfg.get::<bool>("gfx.window.fullscreen").unwrap_or(false);
        let _vsync = cfg.get::<bool>("gfx.window.vsync").unwrap_or(true);
        let window_title = cfg
            .get::<String>("gfx.window.title")
            .unwrap_or_else(|_| "Autograph/autograph_render".to_string());

        let window_builder = winit::WindowBuilder::new()
            .with_title(window_title.clone())
            .with_dimensions((window_width, window_height).into());

        let (instance, window) = create_instance_and_window(&cfg, &events_loop, window_builder);
        let renderer = Renderer::new(instance);

        App {
            events_loop: RefCell::new(events_loop),
            cfg,
            renderer,
            window,
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
                _ => callback(event),
            }
        });
        should_close
    }

    pub fn renderer(&self) -> &Renderer<OpenGlBackend> {
        &self.renderer
    }

    pub fn window(&self) -> &Arc<GlWindow> {
        &self.window
    }
}

pub fn load_environment_config(cfg: &mut config::Config) {
    cfg.merge(config::Environment::with_prefix("GFX")).unwrap();
}

pub fn create_events_loop() -> EventsLoop {
    winit::EventsLoop::new()
}
