//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct FragmentOutputDescription {
    // nothing yet, we just care about the count
}

/*
/// Descriptor for a render target (framebuffer attachment).
#[derive(Copy, Clone, Debug)]
pub struct RenderTargetDescriptor<'a, B: Backend> {
    pub image: &'a B::Image,
}

impl<'a, B: Backend> From<Image<'a, B>> for RenderTargetDescriptor<'a, B> {
    fn from(image: Image<'a, B>) -> Self {
        RenderTargetDescriptor { image: image.0 }
    }
}
*/
