use std::cmp::max;
use std::ptr;
use std::sync::Arc;

use ash::vk;

use crate::device::Device;
use crate::handle::VkHandle;
use crate::image::traits::*;
use crate::image::Dimensions;
use crate::swapchain::Swapchain;

/// Wrapper around an image from a swapchain images.
#[derive(Debug)]
pub struct SwapchainImage {
    swapchain: Arc<Swapchain>,
    image: vk::Image,
    dimensions: (u32, u32),
    array_layers: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    samples: u32,
    layout: vk::ImageLayout,
}

impl SwapchainImage {
    /// Creates a new swapchain image from a raw handle.
    pub(crate) fn new(
        swapchain: &Arc<Swapchain>,
        image: vk::Image,
        index: u32,
        dimensions: (u32, u32),
        array_layers: u32,
        samples: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        layout: vk::ImageLayout,
    ) -> Arc<SwapchainImage> {
        Arc::new(SwapchainImage {
            swapchain: swapchain.clone(),
            image,
            dimensions,
            format,
            usage,
            samples,
            array_layers,
            layout,
        })

        /*// allocate memory for the image from the default allocator
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
        }*/
    }

    /* /// Creates a new image by binding memory to an unbound image.
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
    }*/

    /*/// Destroys this image and returns its associated allocated memory block.
    pub fn destroy(mut self, context: &mut Context) -> Option<AllocatedMemory> {
        if self.should_free_memory {
            context.default_allocator().free_memory(self.memory);
            None
        } else {
            Some(self.memory)
        }
    }*/

    /*/// Creates a new image for the specified swapchain image.
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
    }*/
}

impl ImageDescription for SwapchainImage {
    fn dimensions(&self) -> Dimensions {
        if self.array_layers > 1 {
            Dimensions::Dim2dArray {
                width: self.dimensions.0,
                height: self.dimensions.1,
                array_layers: self.array_layers,
            }
        } else {
            Dimensions::Dim2d {
                width: self.dimensions.0,
                height: self.dimensions.1,
            }
        }
    }

    fn mipmaps_count(&self) -> u32 {
        1
    }

    fn samples(&self) -> u32 {
        self.samples
    }

    fn format(&self) -> vk::Format {
        self.format
    }

    fn usage(&self) -> vk::ImageUsageFlags {
        self.usage
    }
}

/*
impl Resource for Image {
    fn name(&self) -> &str {
        &self.name
    }

    fn last_used_frame(&self) -> FrameNumber {
        self.last_used
    }
}
*/
