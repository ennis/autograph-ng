use std::cmp::max;
use std::ptr;
use std::sync::Arc;

use ash::vk;

use super::unbound::UnboundImage;
use super::dimension::{Dimensions, ImageDimensionInfo};
use super::description::ImageDescription;
use super::{get_texture_mip_map_count, MipmapsCount};

use device::{Device, VkDevice1};
use alloc::{AllocatedMemory, AllocationCreateInfo, Allocator};
use context::{Context, FrameNumber, VkDevice1, FRAME_NONE};
use handle::OwnedHandle;
use resource::Resource;
use sync::SyncGroup;


/// Wrapper around vulkan images.
#[derive(Debug)]
pub struct Image {
    device: Arc<Device>,
    image: vk::Image,
    dimensions: Dimensions,
    format: vk::Format,
    mipmaps_count: u32,
    usage: vk::ImageUsageFlags,
    samples: vk::SampleCountFlags,
    memory: Option<Allocation>,
    should_free_memory: bool,
    last_layout: vk::ImageLayout,
    //last_used: FrameNumber,
    exit_semaphores: SyncGroup<Vec<vk::Semaphore>>,
}

impl Image {
    /// Creates a new image resource, and allocate device memory for it on a suitable pool.
    pub fn new(
        device: &Arc<Device>,
        dimensions: Dimensions,
        mipmaps_count: MipmapsCount,
        samples: vk::SampleCountFlags,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> Arc<Image> {
        let vkd = &context.vkd;

        let unbound_image = UnboundImage::new(
            vkd,
            dimensions,
            mipmaps_count,
            samples,
            format,
            usage,
            vk::ImageLayout::Undefined,
        );
        // allocate memory for the image from the default allocator
        let allocation_create_info = AllocationCreateInfo {
            size: unbound_image.memory_requirements.size,
            alignment: unbound_image.memory_requirements.alignment,
            memory_type_bits: unbound_image.memory_requirements.memory_type_bits,
            preferred_flags: vk::MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
            required_flags: vk::MemoryPropertyFlags::empty(),
        };

        let memory = context
            .default_allocator()
            .allocate_memory(vkd, &allocation_create_info)
            .expect("failed to allocate image memory");

        let image = unbound_image.image.get();
        unsafe {
            vkd.bind_image_memory(image, memory.device_memory, memory.range.start)
                .expect("failed to bind image memory");
        };

        Image {
            image,
            dimensions,
            mipmaps_count: unbound_image.mipmaps_count,
            samples,
            format,
            usage,
            last_layout: vk::ImageLayout::Undefined,
            exit_semaphores: SyncGroup::new(),
            last_used: FRAME_NONE,
            should_free_memory: true,
            memory,
        }
    }

    /// Creates a new image by binding memory to an unbound image.
    pub(crate) fn bind_image_memory(
        vkd: &VkDevice1,
        unbound_image: UnboundImage,
        memory: AllocatedMemory,
    ) -> Image {
        let image = unbound_image.image.get();

        unsafe {
            vkd.bind_image_memory(image, memory.device_memory, memory.range.start)
                .expect("failed to bind image memory");
        };

        Image {
            image,
            dimensions: unbound_image.dimensions,
            mipmaps_count: unbound_image.mipmaps_count,
            samples: unbound_image.samples,
            format: unbound_image.format,
            usage: unbound_image.usage,
            last_layout: vk::ImageLayout::Undefined,
            exit_semaphores: SyncGroup::new(),
            last_used: FRAME_NONE,
            should_free_memory: false,
            memory,
        }
    }

    /// Destroys this image and returns its associated allocated memory block.
    pub fn destroy(mut self, context: &mut Context) -> Option<AllocatedMemory> {
        if self.should_free_memory {
            context.default_allocator().free_memory(self.memory);
            None
        } else {
            Some(self.memory)
        }
    }

    /// Creates a new image for the specified swapchain image.
    pub(crate) fn new_swapchain_image(
        image: OwnedHandle<vk::Image>,
        width: u32,
        height: u32,
        array_layers: u32,
        format: vk::Format,
        usage: vk::ImageUsage,
    ) -> Image {
        Image {
            image,
            dimensions: if array_layers > 1 {
                Dimensions::Dim2dArray {
                    width,
                    height,
                    array_layers,
                }
            } else {
                Dimensions::Dim2d { width, height }
            },
            format,
            mipmaps_count: 1,
            samples: vk::SAMPLE_COUNT_1_BIT,
            memory: None,
            usage: swapchain_create_info.image_usage,
            last_layout: vk::ImageLayout::Undefined,
            last_used: FRAME_NONE,
            exit_semaphores: SyncGroup::new(),
            should_free_memory: false,
        }
    }
}

impl ImageDescription for Image {
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn mipmaps_count(&self) -> u32 {
        self.mipmaps_count
    }

    fn format(&self) -> vk::Format {
        self.format
    }

    fn usage(&self) -> vk::ImageUsageFlags {
        self.usage
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
