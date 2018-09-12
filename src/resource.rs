//! Common resource trait.
//! Buffer and Image resources.
//! `Image`s and `Buffer`s may be in an unallocated (partially valid) state.
//! This is because they are used in the frame graph, where all creation info
//! is not known in advance and allocation is deferred to a later step.

use ash::vk;
use downcast_rs::Downcast;
use slotmap::Key;

//--------------------------------------------------------------------------------------------------
// Resources

/// Trait representing the shared functionality and properties of resources (buffers and images).
pub trait Resource: Downcast {
    fn name(&self) -> &str;
}
impl_downcast!(Resource);

//--------------------------------------------------------------------------------------------------
// Buffer

/// A buffer resource. Possibly virtual (not yet allocated).
/// Note that it is clonable, but this does not extend its lifetime.
#[derive(Debug)]
pub struct Buffer {
    /// Name of the resource. May not uniquely identify the resource;
    pub(crate) name: String,
    /// Buffer creation info. Some properties are inferred from the dependency graph.
    // FIXME this is not what should be kept in the object.
    pub(crate) create_info: vk::BufferCreateInfo,
    /// Buffer resource + associated memory allocation, None if not yet allocated.
    /// A not-yet-allocated resource is called "virtual"
    pub(crate) buffer: Option<vk::Buffer>,
}

impl Buffer {
    /// Creates a new unallocated buffer (virtual buffer).
    pub(crate) fn new(name: impl Into<String>, create_info: &vk::BufferCreateInfo) -> Buffer {
        Buffer {
            name: name.into(),
            create_info: create_info.clone(),
            buffer: None,
        }
    }

    pub fn create_info(&self) -> &vk::BufferCreateInfo {
        &self.create_info
    }

    pub(crate) fn is_allocated(&self) -> bool {
        self.buffer.is_some()
    }
}

impl Resource for Buffer {
    fn name(&self) -> &str {
        &self.name
    }
}

//--------------------------------------------------------------------------------------------------
// Image

/// An image resource.
#[derive(Debug)]
pub struct Image {
    /// Name of the resource. May not uniquely identify the resource;
    pub(crate) name: String,
    /// Image creation info.
    // FIXME this is not what should be kept in the object.
    pub(crate) create_info: vk::ImageCreateInfo,
    /// Image resource + associated memory allocation, `None` if not yet allocated.
    pub(crate) image: Option<vk::Image>,
}

impl Image {
    /// Creates a new unallocated image (virtual image).
    pub(crate) fn new(name: impl Into<String>, create_info: &vk::ImageCreateInfo) -> Image {
        Image {
            name: name.into(),
            create_info: create_info.clone(),
            image: None,
        }
    }

    pub fn create_info(&self) -> &vk::ImageCreateInfo {
        &self.create_info
    }

    pub(crate) fn is_allocated(&self) -> bool {
        self.image.is_some()
    }
}

impl Resource for Image {
    fn name(&self) -> &str {
        &self.name
    }
}
