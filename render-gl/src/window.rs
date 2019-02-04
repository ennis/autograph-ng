use crate::backend::OpenGlInstance;
use crate::swapchain::SwapchainInner;
use config::Config;
use glutin;
use glutin::GlContext;
use winit::{EventsLoop, WindowBuilder};

impl SwapchainInner for glutin::GlWindow {
    fn size(&self) -> (u32, u32) {
        self.get_inner_size().unwrap().into()
    }

    fn present(&self) {
        self.swap_buffers().expect("failed to swap buffers")
    }
}

pub fn create_instance_and_window(
    cfg: &Config,
    events_loop: &EventsLoop,
    window_builder: WindowBuilder,
) -> OpenGlInstance {
    // TODO get config from config file
    let context_builder = glutin::ContextBuilder::new()
        .with_gl_profile(glutin::GlProfile::Core)
        .with_gl_debug_flag(true)
        .with_vsync(true)
        .with_srgb(true)
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (4, 6)));

    let window = glutin::GlWindow::new(window_builder, context_builder, events_loop)
        .expect("unable to create window");

    // Make current the OpenGL context associated to the window
    // and load function pointers
    unsafe { window.make_current() }.unwrap();

    let glfns = crate::api::Gl::load_with(|symbol| {
        let ptr = window.get_proc_address(symbol) as *const _;
        //debug!("getProcAddress {} -> {:?}", symbol, ptr);
        ptr
    });

    OpenGlInstance::with_gl(cfg, glfns, Box::new(window))
}
