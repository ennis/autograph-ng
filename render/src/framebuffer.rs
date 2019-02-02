use crate::image::Image;

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
pub struct FragmentOutputDescription {
    // nothing yet, we just care about the count
}

/// Descriptor for a render target (framebuffer attachment).
#[derive(Copy, Clone, Debug)]
pub struct RenderTargetDescriptor<'a> {
    ///
    pub image: Image<'a>
}

impl<'a> From<Image<'a>> for RenderTargetDescriptor<'a> {
    fn from(image: Image<'a>) -> Self {
        RenderTargetDescriptor{
            image
        }
    }
}
