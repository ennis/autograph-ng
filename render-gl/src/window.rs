use crate::backend::OpenGlInstance;
use config::Config;
use glutin;
use glutin::GlWindow;
use std::sync::Arc;
use winit::{EventsLoop, WindowBuilder};

pub fn create_instance_and_window(
    cfg: &Config,
    events_loop: &EventsLoop,
    window_builder: WindowBuilder,
) -> (OpenGlInstance, Arc<GlWindow>) {
    // TODO get config from config file
    let context_builder = glutin::ContextBuilder::new()
        .with_gl_profile(glutin::GlProfile::Core)
        .with_gl_debug_flag(true)
        .with_vsync(true)
        .with_srgb(true)
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (4, 6)));

    let window = Arc::new(
        glutin::GlWindow::new(window_builder, context_builder, events_loop)
            .expect("unable to create window"),
    );

    let inst = OpenGlInstance::from_gl_window(cfg, window.clone());
    (inst, window)
}
