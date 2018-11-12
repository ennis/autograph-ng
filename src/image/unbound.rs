use std::cmp::max;
use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::device::{Device, SharingMode, VkDevice1};
use crate::handle::VkHandle;
use crate::image::traits::{Image, ImageDescription};
use crate::image::{get_texture_mip_map_count, Dimensions, ImageExtentsAndType, MipmapsCount};
use crate::resource::Resource;
//use crate::sync::SyncGroup;

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
        sharing: SharingMode,
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

        let (sharing_mode, p_queue_family_indices, queue_family_index_count) = match sharing {
            SharingMode::Exclusive => (vk::SharingMode::Exclusive, ptr::null(), 0),
            SharingMode::Concurrent(ref families) => (
                vk::SharingMode::Concurrent,
                families.as_ptr(),
                families.len() as u32,
            ),
        };

        let create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::ImageCreateInfo,
            p_next: ptr::null(),
            flags: vk::ImageCreateFlags::default(),
            image_type: extents_and_type.type_,
            format,
            extent: extents_and_type.extent,
            mip_levels,
            array_layers: extents_and_type.array_layers,
            samples: sample_bits,
            tiling: vk::ImageTiling::Optimal,
            usage,
            sharing_mode,
            queue_family_index_count,
            p_queue_family_indices,
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

    pub fn memory_requirements(&self) -> vk::MemoryRequirements {
        self.memory_requirements
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

    fn usage(&self) -> vk::ImageUsageFlags {
        self.usage
    }
}

/*
impl Image for UnboundImage {
    fn device(&self) -> &Device {
        self.device
    }

    fn layout(&self) -> vk::ImageLayout {
        self.layout
    }
}
*/
