//! Common resource trait.
//! Buffer and Image resources.
//! `Image`s and `Buffer`s may be in an unallocated (partially valid) state.
//! This is because they are used in the frame graph, where all creation info
//! is not known in advance and allocation is deferred to a later step.
//!

use std::mem;

use ash::version::DeviceV1_0;
use ash::vk;
use downcast_rs::Downcast;
use slotmap::Key;

use context::{FrameNumber, VkDevice1, FRAME_NONE};
use sync::{FrameSync, SyncGroup};

//--------------------------------------------------------------------------------------------------
// Resources

/// Trait representing the shared functionality and properties of resources (buffers and images).
pub trait Resource: Downcast {
    /// Gets the name of the resource.
    /// Note that the name does not uniquely identifies a resource,
    /// as it does not need to be unique among all resources.
    fn name(&self) -> &str;

    /// The frame in which the resource was last used.
    fn last_used_frame(&self) -> FrameNumber;
}
impl_downcast!(Resource);

//--------------------------------------------------------------------------------------------------
// Buffer

/// A buffer resource. Possibly virtual (not yet allocated).
/// Note that it is cloneable, but this does not extend its lifetime.
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
    /// Last used frame. Can be `never`
    pub(crate) last_used: FrameNumber,
    /// Used for synchronization between frames.
    pub(crate) exit_semaphores: SyncGroup<Vec<vk::Semaphore>>,
}

impl Buffer {
    /// Creates a new unallocated buffer (virtual buffer).
    pub(crate) fn new(name: impl Into<String>, create_info: &vk::BufferCreateInfo) -> Buffer {
        Buffer {
            name: name.into(),
            create_info: create_info.clone(),
            buffer: None,
            last_used: FRAME_NONE,
            exit_semaphores: SyncGroup::new(),
        }
    }

    /// Sets a list of semaphores signalled by the resource when the frame ends.
    pub(crate) fn set_exit_semaphores(
        &mut self,
        semaphores: Vec<vk::Semaphore>,
        frame_sync: &mut FrameSync,
        vkd: &VkDevice1,
    ) {
        self.exit_semaphores
            .enqueue(semaphores, frame_sync, |semaphores| {
                for sem in semaphores {
                    unsafe {
                        vkd.destroy_semaphore(sem, None);
                    }
                }
            });
    }

    pub fn create_info(&self) -> &vk::BufferCreateInfo {
        &self.create_info
    }

    pub fn size(&self) -> vk::DeviceSize {
        self.create_info.size
    }

    pub(crate) fn is_allocated(&self) -> bool {
        self.buffer.is_some()
    }
}

impl Resource for Buffer {
    fn name(&self) -> &str {
        &self.name
    }

    fn last_used_frame(&self) -> FrameNumber {
        self.last_used
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
    /// Last used frame. Can be `never`
    pub(crate) last_used: FrameNumber,
    /// If the image is part of the swapchain, that's its index. Otherwise, None.
    pub(crate) swapchain_index: Option<u32>,
    /// Used for synchronization between frames.
    pub(crate) exit_semaphores: SyncGroup<Vec<vk::Semaphore>>,
}

impl Image {
    /// Creates a new unallocated image (virtual image).
    pub(crate) fn new(name: impl Into<String>, create_info: &vk::ImageCreateInfo) -> Image {
        Image {
            name: name.into(),
            create_info: create_info.clone(),
            image: None,
            swapchain_index: None,
            last_used: FRAME_NONE,
            exit_semaphores: SyncGroup::new(),
        }
    }

    /// Creates a new image for the specified swapchain image.
    pub(crate) fn new_swapchain_image(
        name: impl Into<String>,
        image: vk::Image,
        swapchain_index: u32,
    ) -> Image {
        Image {
            name: name.into(),
            create_info: unsafe { mem::zeroed() }, // FIXME HARDER
            image: Some(image),
            swapchain_index: Some(swapchain_index),
            last_used: FRAME_NONE,
            exit_semaphores: SyncGroup::new(),
        }
    }

    /// Sets a list of semaphores signalled by the resource when the frame ends.
    pub(crate) fn set_exit_semaphores(
        &mut self,
        semaphores: Vec<vk::Semaphore>,
        frame_sync: &mut FrameSync,
        vkd: &VkDevice1,
    ) {
        self.exit_semaphores
            .enqueue(semaphores, frame_sync, |semaphores| {
                for sem in semaphores {
                    unsafe {
                        vkd.destroy_semaphore(sem, None);
                    }
                }
            });
    }

    /// Returns the dimensions of the image.
    pub fn dimensions(&self) -> (u32, u32, u32) {
        (
            self.create_info.extent.width,
            self.create_info.extent.height,
            self.create_info.extent.depth,
        )
    }

    /// Returns the format of the image.
    pub fn format(&self) -> vk::Format {
        self.create_info.format
    }

    pub fn create_info(&self) -> &vk::ImageCreateInfo {
        &self.create_info
    }

    pub(crate) fn is_allocated(&self) -> bool {
        self.image.is_some()
    }

    pub(crate) fn is_swapchain_image(&self) -> bool {
        self.swapchain_index.is_some()
    }
}

impl Resource for Image {
    fn name(&self) -> &str {
        &self.name
    }

    fn last_used_frame(&self) -> FrameNumber {
        self.last_used
    }
}
