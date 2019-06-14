use crate::Backend;

//--------------------------------------------------------------------------------------------------
/// Swapchains.
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
#[repr(transparent)]
pub struct Swapchain<'a, B: Backend>(pub &'a B::Swapchain);

impl<'a, B: Backend> Swapchain<'a, B> {
    pub fn size(&self) -> (u32, u32) {
        crate::traits::Swapchain::size(self.0)
    }
}
