use autograph_render::traits;
use glutin::GlWindow;
use std::fmt;
use std::sync::Arc;

/// Represents an OpenGL "swapchain".
///
/// OpenGL does not have the concept of "swapchains": this is typically handled by the
/// underlying window system. This type wraps around window handles and provides an interface
/// for getting the size of the swapchain (default framebuffer) and present an image to the screen
/// (swap buffers).
pub struct GlSwapchain {
    pub(crate) window: Arc<GlWindow>,
}

impl fmt::Debug for GlSwapchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Swapchain {{..}}")
    }
}

impl traits::Swapchain for GlSwapchain {
    fn size(&self) -> (u32, u32) {
        self.window.get_inner_size().unwrap().into()
    }
}
