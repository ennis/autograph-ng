use downcast_rs::impl_downcast;
pub use downcast_rs::Downcast;
use std::fmt::Debug;

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
/// Trait implemented by backend descriptor set layout objects.
pub trait DescriptorSetLayout: Downcast + Debug {}
/// Trait implemented by backend shader module objects.
pub trait ShaderModule: Downcast + Debug {}
/// Trait implemented by backend graphics pipeline objects.
pub trait GraphicsPipeline: Downcast + Debug {}
/// Trait implemented by backend descriptor set objects.
pub trait DescriptorSet: Downcast + Debug {}

pub trait Arena: Downcast + Sync {}

// allow unchecked downcasting of trait objects: we guarantee that the objects passed to the backend
// are of the correct type.
impl_downcast!(Swapchain);
impl_downcast!(Buffer);
impl_downcast!(Image);
impl_downcast!(Framebuffer);
impl_downcast!(DescriptorSetLayout);
impl_downcast!(ShaderModule);
impl_downcast!(GraphicsPipeline);
impl_downcast!(DescriptorSet);
impl_downcast!(Arena);
