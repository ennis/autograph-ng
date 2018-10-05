use std::cmp::max;
use std::ptr;
use std::sync::Arc;

use ash::vk;

use super::dimension::Dimensions;
use super::description::ImageDescription;
use super::{get_texture_mip_map_count, Dimensions, ImageDimensionInfo, MipmapsCount};

use device::{Device, VkDevice1};
use context::{Context, FrameNumber, VkDevice1, FRAME_NONE};
use handle::OwnedHandle;
use resource::Resource;
use sync::SyncGroup;


/// Wrapper around an image without associated memory.
pub(super) struct UnboundImage {
    pub(super) device: Arc<Device>,
    pub(super) image: vk::Image,
    pub(super) dimensions: Dimensions,
    pub(super) mipmaps_count: u32,
    pub(super) format: vk::Format,
    pub(super) usage: vk::ImageUsageFlags,
    pub(super) memory_requirements: vk::MemoryRequirements,
    pub(super) samples: vk::SampleCountFlags,
}

impl UnboundImage {
    pub fn new(
        device: &Arc<Device>,
        dimensions: Dimensions,
        mipmaps_count: MipmapsCount,
        samples: vk::SampleCountFlags,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        initial_layout: vk::InitialLayout,
    ) -> UnboundImage {

        let dim_info = dimensions.to_image_dimension_info();

        let mip_levels = match mipmaps_count {
            MipmapsCount::One => 1,
            MipmapsCount::Specific(num) => num,
            MipmapsCount::Log2 => {
                let size = max(
                    max(dim_info.extent.width, dim_info.extent.height),
                    dim_info.extent.depth,
                );
                get_texture_mip_map_count(size)
            }
        };

        let create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::ImageCreateInfo,
            p_next: ptr::null(),
            flags: vk::ImageCreateFlags::default(),
            image_type: vk::ImageType::Type2d,
            format,
            extent: dim_info.extent,
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
            let image = device.pointers()
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
            }
        }
    }
}