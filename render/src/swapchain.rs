use crate::traits;

//--------------------------------------------------------------------------------------------------

/// Swapchains.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Swapchain<'a>(pub &'a dyn traits::Swapchain);

impl<'a> Swapchain<'a> {
    /// Returns the current size of the swapchain.
    pub fn size(&self) -> (u32, u32) {
        traits::Swapchain::size(self.0)
    }
}
