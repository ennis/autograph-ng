use crate::traits;

/// Framebuffer.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Framebuffer<'a>(pub &'a dyn traits::Framebuffer);

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
pub struct FragmentOutputDescription {
    // nothing yet, we just care about the count
}
