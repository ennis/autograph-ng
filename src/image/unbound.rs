use std::cmp::max;
use std::ptr;
use std::sync::Arc;

use ash::vk;

use crate::image::traits::{Image, ImageDescription};
use crate::image::{get_texture_mip_map_count, Dimensions, ImageExtentsAndType, MipmapsCount};

use context::{Context, FrameNumber, VkDevice1, FRAME_NONE};
use device::{Device, VkDevice1};
use handle::VkHandle;
use resource::Resource;
use sync::SyncGroup;

/// Wrapper around an image without associated memory.
pub struct UnboundImage {
    device: Arc<Device>,
    image: vk::Image,
    dimensions: Dimensions,
    mipmaps_count: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    memory_requirements: vk::MemoryRequirements,
    samples: u32,
    layout: vk::ImageLayout,
}

impl UnboundImage {
    pub fn new(
        device: &Arc<Device>,
        dimensions: Dimensions,
        mipmaps_count: MipmapsCount,
        samples: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        initial_layout: vk::ImageLayout,
    ) -> UnboundImage {
        let extents_and_type = dimensions.to_image_extents_and_type();

        assert!(
            samples.is_power_of_two(),
            "sample count must be a power of two"
        );
        let sample_bits = match samples {
            1 => vk::SAMPLE_COUNT_1_BIT,
            2 => vk::SAMPLE_COUNT_2_BIT,
            4 => vk::SAMPLE_COUNT_4_BIT,
            8 => vk::SAMPLE_COUNT_8_BIT,
            16 => vk::SAMPLE_COUNT_16_BIT,
            32 => vk::SAMPLE_COUNT_32_BIT,
            64 => vk::SAMPLE_COUNT_64_BIT,
            _ => panic!("unsupported sample count"),
        };

        let mip_levels = match mipmaps_count {
            MipmapsCount::One => 1,
            MipmapsCount::Specific(num) => num,
            MipmapsCount::Log2 => {
                let size = max(
                    max(
                        extents_and_type.extent.width,
                        extents_and_type.extent.height,
                    ),
                    extents_and_type.extent.depth,
                );
                get_texture_mip_map_count(size)
            }
        };

        let create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::ImageCreateInfo,
            p_next: ptr::null(),
            flags: vk::ImageCreateFlags::default(),
            image_type: extents_and_type.type_,
            format,
            extent: extents_and_type.extent,
            mip_levels,
            array_layers: dim_info.array_layers,
            samples,
            tiling: vk::ImageTiling::Optimal,
            usage,
            sharing_mode: vk::SharingMode::Exclusive,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            initial_layout,
        };

        unsafe {
            let image = device
                .pointers()
                .create_image(&create_info, None)
                .expect("could not create image");

            let memory_requirements = device.pointers().get_image_memory_requirements(image);

            UnboundImage {
                device: device.clone(),
                image,
                dimensions,
                mipmaps_count: mip_levels,
                format,
                samples,
                usage,
                memory_requirements,
                layout: initial_layout,
            }
        }
    }
}

impl ImageDescription for UnboundImage {
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn mipmaps_count(&self) -> u32 {
        self.mipmaps_count
    }

    fn samples(&self) -> u32 {
        self.samples
    }

    fn format(&self) -> vk::Format {
        self.format
    }

    fn usage(&self) -> ImageUsageFlags {
        self.usage
    }
}

impl Image for UnboundImage {
    fn device(&self) -> &Device {
        unimplemented!()
    }

    fn layout(&self) -> vk::ImageLayout {
        unimplemented!()
    }
}
