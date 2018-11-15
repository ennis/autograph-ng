/*
/// A buffer resource, written once at creation and then immutable.
#[derive(Debug)]
pub struct ImmutableBuffer {
    device: Arc<Device>,
    usage: vk::BufferUsageFlags,
    buffer: VkHandle<vk::Buffer>,
    memory: AllocatedMemory,
}

impl Buffer {
    /// Creates a new buffer.
    pub fn new(device: &Arc<Device>, size: u64, usage: vk::BufferUsageFlags) -> Buffer
    {
        let unbound = UnboundBuffer::new(device, size, usage);

        Buffer {
            device: device.clone(),
            usage,
            buffer: None,
        }
    }

    pub fn bind_buffer_memory(
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
        }
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

}
*/
