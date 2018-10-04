//! Buffers
//!
use std::ptr;

use ash::vk;

use alloc::{AllocatedMemory, AllocationCreateInfo, Allocator};
use context::{Context, FrameNumber, VkDevice1, FRAME_NONE};
use handle::OwnedHandle;
use resource::Resource;
use sync::SyncGroup;

pub trait BufferDescription {
    fn size(&self) -> u64;
    fn usage(&self) -> vk::BufferUsageFlags;
}

/// A buffer resource without device memory bound to it.
struct UnboundBuffer {
    buffer: OwnedHandle<vk::Buffer>,
    size: u64,
    usage: vk::BufferUsage,
    memory_requirements: vk::MemoryRequirements,
}

impl UnboundBuffer {
    fn new(vkd: &VkDevice1, size: u64, usage: vk::BufferUsageFlags) -> UnboundBuffer {
        let create_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BufferCreateInfo,
            p_next: ptr::null(),
            size,
            usage,
            flags: vk::BufferCreateFlags::empty(),
            sharing_mode: vk::SharingMode::Exclusive, // FIXME
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
        };

        unsafe {
            let buffer = vkd
                .create_buffer(&create_into, None)
                .expect("could not create buffer");
            let memory_requirements = vkd.get_buffer_memory_requirements(buffer);

            UnboundBuffer {
                buffer: OwnedHandle(buffer),
                size,
                usage,
                memory_requirements,
            }
        }
    }
}

/// A buffer resource.
#[derive(Debug)]
pub struct Buffer {
    /// Buffer creation info.
    size: u64,

    /// Buffer usage.
    usage: vk::BufferUsageFlags,

    /// Buffer resource + associated memory allocation.
    buffer: OwnedHandle<vk::Buffer>,

    /// Device memory bound to the buffer.
    memory: AllocatedMemory,

    /// Specifies whether the memory should be freed when the buffer is destroyed.
    should_free_memory: bool,

    /// Last used frame. Can be `never`
    last_used: FrameNumber,

    /// Used for synchronization between frames.
    exit_semaphores: SyncGroup<Vec<vk::Semaphore>>,
}

impl Buffer {
    /// Creates a new buffer.
    pub(crate) fn new(context: &mut Context, size: u64, usage: vk::BufferUsageFlags) -> Buffer {
        let vkd = &context.vkd;
        let unbound = UnboundBuffer::new(vkd, size, usage);

        //let memory = context.default_allocator().

        Buffer {
            size,
            usage,
            buffer: None,
            last_used: FRAME_NONE,
            exit_semaphores: SyncGroup::new(),
        }
    }

    pub(crate) fn bind_buffer_memory(
        vkd: &VkDevice1,
        unbound: UnboundBuffer,
        memory: AllocatedMemory,
    ) -> Buffer {
        unsafe {
            vkd.bind_buffer_memory(
                unbound.buffer.get(),
                memory.device_memory,
                memory.range.start,
            );
        };

        Buffer {
            buffer: unbound.buffer,
            size: unbound.size,
            usage: unbound.usage,
            last_used: FRAME_NONE,
            exit_semaphores: SyncGroup::new(),
        }
    }

    /*/// Sets a list of semaphores signalled by the resource when the frame ends.
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
    }*/

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
