//! Common resource trait.
//! Buffer and Image resources.
//! `Image`s and `Buffer`s may be in an unallocated (partially valid) state.
//! This is because they are used in the frame graph, where all creation info
//! is not known in advance and allocation is deferred to a later step.
//!

use std::cell::Cell;
use std::mem;
use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;
use downcast_rs::Downcast;
use slotmap::Key;

use alloc::Allocation;
use context::{FrameNumber, VkDevice1, FRAME_NONE};
use handle::OwningHandle;
use sync::{FrameSync, SyncGroup};

//--------------------------------------------------------------------------------------------------
// Resources

/// Trait representing the shared functionality and properties of resources (buffers and images).
pub trait Resource {
    type CreateInfo: Clone;

    /// Gets the name of the resource.
    /// Note that the name does not uniquely identifies a resource,
    /// as it does not need to be unique among all resources.
    fn name(&self) -> &str;

    /// The frame in which the resource was last used.
    fn last_used_frame(&self) -> FrameNumber;

    fn create_info(&self) -> &Self::CreateInfo;
}

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
    pub(crate) buffer: Option<OwningHandle<vk::Buffer>>,
    /// Last used frame. Can be `never`
    pub(crate) last_used: FrameNumber,
    /// Used for synchronization between frames.
    pub(crate) exit_semaphores: SyncGroup<Vec<vk::Semaphore>>,
}

impl Buffer {
    /// Creates a new unallocated buffer (virtual buffer).
    pub(crate) fn new(name: impl Into<String>, create_info: vk::BufferCreateInfo) -> Buffer {
        Buffer {
            name: name.into(),
            create_info,
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

    pub fn size(&self) -> vk::DeviceSize {
        self.create_info.size
    }

    pub(crate) fn is_allocated(&self) -> bool {
        self.buffer.is_some()
    }
}

impl Resource for Buffer {
    type CreateInfo = vk::BufferCreateInfo;

    fn name(&self) -> &str {
        &self.name
    }

    fn last_used_frame(&self) -> FrameNumber {
        self.last_used
    }

    fn create_info(&self) -> &vk::BufferCreateInfo {
        &self.create_info
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
    /// Image resource +
    pub(crate) image: Option<OwningHandle<vk::Image>>,
    /// associated memory allocation, `None` if not allocated by us.
    pub(crate) allocation: Option<Allocation>,
    /// Last known layout.
    pub(crate) last_layout: Cell<vk::ImageLayout>,
    /// Last used frame. Can be `never`
    pub(crate) last_used: FrameNumber,
    /// If the image is part of the swapchain, that's its index. Otherwise, None.
    pub(crate) swapchain_index: Option<u32>,
    /// Used for synchronization between frames.
    pub(crate) exit_semaphores: SyncGroup<Vec<vk::Semaphore>>,
}

impl Image {
    /// Creates a new unallocated image (virtual image).
    pub(crate) fn new(name: impl Into<String>, create_info: vk::ImageCreateInfo) -> Image {
        Image {
            name: name.into(),
            create_info,
            image: None,
            swapchain_index: None,
            last_layout: Cell::new(vk::ImageLayout::General),
            last_used: FRAME_NONE,
            exit_semaphores: SyncGroup::new(),
        }
    }

    /// Creates a new image for the specified swapchain image.
    pub(crate) fn new_swapchain_image(
        name: impl Into<String>,
        swapchain_create_info: &vk::SwapchainCreateInfoKHR,
        image: OwningHandle<vk::Image>,
        swapchain_index: u32,
    ) -> Image {
        Image {
            name: name.into(),
            create_info: vk::ImageCreateInfo {
                s_type: vk::StructureType::ImageCreateInfo,
                p_next: ptr::null(),
                flags: vk::ImageCreateFlags::empty(),
                image_type: vk::ImageType::Type2d,
                format: swapchain_create_info.image_format,
                extent: vk::Extent3D {
                    width: swapchain_create_info.image_extent.width,
                    height: swapchain_create_info.image_extent.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: swapchain_create_info.image_array_layers,
                samples: vk::SAMPLE_COUNT_1_BIT,
                tiling: vk::ImageTiling::Optimal,
                usage: swapchain_create_info.image_usage,
                sharing_mode: swapchain_create_info.image_sharing_mode,
                queue_family_index_count: 0,
                p_queue_family_indices: ptr::null(),
                initial_layout: vk::ImageLayout::Undefined,
            },
            image: Some(image),
            swapchain_index: Some(swapchain_index),
            last_layout: Cell::new(vk::ImageLayout::Undefined),
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

    /// Returns the usage flags of the image.
    pub fn usage(&self) -> vk::ImageUsageFlags {
        self.create_info.usage
    }

    pub(crate) fn is_allocated(&self) -> bool {
        self.image.is_some()
    }

    pub(crate) fn is_swapchain_image(&self) -> bool {
        self.swapchain_index.is_some()
    }

    pub(crate) fn last_layout(&self) -> vk::ImageLayout {
        self.last_layout.get()
    }
}

impl Resource for Image {
    type CreateInfo = vk::ImageCreateInfo;

    fn name(&self) -> &str {
        &self.name
    }

    fn last_used_frame(&self) -> FrameNumber {
        self.last_used
    }

    fn create_info(&self) -> &vk::ImageCreateInfo {
        &self.create_info
    }
}
