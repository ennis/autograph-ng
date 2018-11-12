use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{Buffer, BufferDescription};
use crate::device::Device;
use crate::handle::VkHandle;
use crate::memory::{AllocationCreateInfo, MemoryBlock, MemoryPool};
use crate::resource::Resource;

/// A buffer resource without device memory bound to it.
pub struct UnboundBuffer {
    device: Arc<Device>,
    buffer: VkHandle<vk::Buffer>,
    size: u64,
    usage: vk::BufferUsageFlags,
    memory_requirements: vk::MemoryRequirements,
}

impl UnboundBuffer {
    pub fn new(device: &Arc<Device>, size: u64, usage: vk::BufferUsageFlags) -> UnboundBuffer {
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
            let buffer = device
                .pointers()
                .create_buffer(&create_info, None)
                .expect("could not create buffer");
            let memory_requirements = device.pointers().get_buffer_memory_requirements(buffer);

            UnboundBuffer {
                device: device.clone(),
                buffer: VkHandle::new(buffer),
                size,
                usage,
                memory_requirements,
            }
        }
    }
}

impl BufferDescription for UnboundBuffer {
    fn size(&self) -> u64 {
        self.size
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        self.usage
    }
}

impl Buffer for UnboundBuffer {
    fn device(&self) -> &Device {
        &self.device
    }
}
