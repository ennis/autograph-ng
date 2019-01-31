use downcast_rs::impl_downcast;
pub use downcast_rs::Downcast;
use std::fmt::Debug;
use crate::descriptor::Descriptor;
use crate::pipeline::Viewport;
use crate::pipeline::ScissorRect;
use crate::vertex::VertexBufferDescriptor;
use crate::vertex::IndexBufferDescriptor;
use crate::framebuffer::RenderTargetDescriptor;

/// Trait implemented by backend swapchain objects.
pub trait Swapchain: Downcast + Debug {
    fn size(&self) -> (u32, u32);
}

/// Trait implemented by backend buffer objects.
pub trait Buffer: Downcast + Debug {
    fn size(&self) -> u64;
}

/// Trait implemented by backend image objects.
pub trait Image: Downcast + Debug {}
/// Trait implemented by backend framebuffer objects.
pub trait Framebuffer: Downcast + Debug {}
/// Trait implemented by backend shader module objects.
pub trait ShaderModule: Downcast + Debug {}
/// Trait implemented by backend graphics pipeline objects.
pub trait GraphicsPipeline: Downcast + Debug {}
///
pub trait PipelineArguments: Downcast + Debug {
    unsafe fn set_arguments<'a>(&self, index: usize, arguments: &'a dyn PipelineArguments);
    unsafe fn set_descriptor<'a>(&self, index: usize, descriptor: Descriptor<'a>);
    unsafe fn set_viewport(&self, index: usize, viewport: &Viewport);
    unsafe fn set_scissor(&self, index: usize, scissor: &ScissorRect);
    unsafe fn set_vertex_buffer<'a>(&self, index: usize, vertex_buffer: VertexBufferDescriptor<'a, '_>);
    unsafe fn set_index_buffer<'a>(&self, index: usize, vertex_buffer: Option<IndexBufferDescriptor<'a>>);
    unsafe fn set_render_target<'a>(&self, index: usize, render_target: RenderTargetDescriptor<'a>);
}

/// A reference to host data that is used in pipeline arguments.
pub trait HostReference: Downcast + Debug {}

pub trait Arena: Downcast + Sync {}

// allow unchecked downcasting of trait objects: we guarantee that the objects passed to the backend
// are of the correct type.
impl_downcast!(Swapchain);
impl_downcast!(Buffer);
impl_downcast!(Image);
impl_downcast!(Framebuffer);
impl_downcast!(PipelineArguments);
impl_downcast!(ShaderModule);
impl_downcast!(GraphicsPipeline);
impl_downcast!(HostReference);
impl_downcast!(Arena);
