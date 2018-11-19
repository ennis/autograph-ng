use super::OpenGlBackend;

use config::Config;
use glutin;
use glutin::{Api, GlContext, GlWindow};
use winit::{EventsLoop, WindowBuilder};

impl OpenGlBackend {}

pub fn create_backend_and_window(
    cfg: &Config,
    events_loop: &EventsLoop,
    window_builder: WindowBuilder,
) -> OpenGlBackend {
    // TODO get config from config file
    let context_builder = glutin::ContextBuilder::new()
        .with_gl_profile(glutin::GlProfile::Core)
        .with_gl_debug_flag(true)
        .with_vsync(true)
        .with_srgb(true)
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (4, 6)));

    let window = glutin::GlWindow::new(window_builder, context_builder, events_loop)
        .expect("unable to create window");

    OpenGlBackend::with_gl_window(cfg, window)
}
