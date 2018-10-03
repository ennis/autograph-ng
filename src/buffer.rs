//! Buffers
//!


use ash::vk;

use handle::OwningHandle;
use sync::SyncGroup;
use context::{FrameNumber, VkDevice1, FRAME_NONE};
use resource::Resource;

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