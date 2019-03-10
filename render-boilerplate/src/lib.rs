//! Boilerplate code for creating a window and an OpenGL context with winit/glutin.
extern crate image as img;
use self::img::GenericImageView;
use autograph_render::*;
use autograph_render_gl::{create_instance_and_window, InstanceConfig, OpenGlBackend};
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

/*
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

impl error::Error for ImageLoadError {}*/

/*
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
        MipmapsOption::One,
        1,
        ImageUsageFlags::SAMPLED,
        bytes,
    ))
}*/

pub struct App {
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
        let cfg = InstanceConfig::default();

        let events_loop = create_events_loop();
        let window_width = 640;
        let window_height = 480;
        let window_title = "autograph";

        let window_builder = winit::WindowBuilder::new()
            .with_title(window_title)
            .with_dimensions((window_width, window_height).into());

        let (instance, window) = create_instance_and_window(&cfg, &events_loop, window_builder);
        let renderer = Renderer::new(instance);

        App {
            events_loop: RefCell::new(events_loop),
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

pub fn create_events_loop() -> EventsLoop {
    winit::EventsLoop::new()
}
