use crate::handle;

//--------------------------------------------------------------------------------------------------
/// Swapchains.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Swapchain<'a>(pub handle::Swapchain<'a>);
