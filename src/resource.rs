//! Common resource trait.
//! Buffer and Image resources.
//! `ImageResource`s and `BufferResource`s may be in an unallocated (partially valid) state.
//! This is because they are used in the frame graph, where all creation info
//! is not known in advance and allocation is deferred to a later step.

use downcast_rs::Downcast;
use ash::vk;

/// Trait representing the shared functionality and properties of resources (buffers and images).
pub trait Resource: Downcast
{
    fn name(&self) -> &str;
}
impl_downcast!(Resource);

/// A buffer resource. Possibly virtual (not yet allocated).
pub struct BufferResource
{
    /// Name of the resource. May not uniquely identify the resource;
    pub(crate) name: String,
    /// Buffer creation info. Some properties are inferred from the dependency graph.
    pub(crate) create_info: vk::BufferCreateInfo,
    /// Buffer resource + associated memory allocation, None if not yet allocated.
    /// A not-yet-allocated resource is called "virtual"
    pub(crate) buffer: Option<vk::Buffer>,
}

impl BufferResource
{
    /// Creates a new unallocated buffer (virtual buffer).
    pub(crate) fn new(name: impl Into<String>, create_info: &vk::BufferCreateInfo) -> BufferResource {
        BufferResource {
            name: name.into(),
            create_info: create_info.clone(),
            buffer: None,
        }
    }

    pub(crate) fn is_allocated(&self) -> bool {
        self.buffer.is_some()
    }
}

impl Resource for BufferResource
{
    fn name(&self) -> &str {
        &self.name
    }
}

/// An image resource.
pub struct ImageResource
{
    /// Name of the resource. May not uniquely identify the resource;
    pub(crate) name: String,
    /// Buffer creation info.
    pub(crate) create_info: vk::ImageCreateInfo,
    /// Image resource + associated memory allocation, `None` if not yet allocated.
    pub(crate) image: Option<vk::Image>,
}

impl ImageResource
{
    /// Creates a new unallocated image (virtual image).
    pub(crate) fn new(name: impl Into<String>, create_info: &vk::ImageCreateInfo) -> ImageResource {
        ImageResource {
            name: name.into(),
            create_info: create_info.clone(),
            image: None,
        }
    }
}

impl Resource for ImageResource
{
    fn name(&self) -> &str {
        &self.name
    }
}
