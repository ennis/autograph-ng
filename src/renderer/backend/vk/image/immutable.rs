use std::cell::Cell;
use std::cmp::max;
use std::ptr;
use std::sync::Arc;

use ash::vk;

use crate::device::{Device, DeviceBoundObject, FrameNumber, FrameSynchronizedObject, SharingMode};
use crate::handle::VkHandle;
use crate::image::traits::*;
use crate::image::unbound::UnboundImage;
use crate::image::*;
use crate::sync::FrameLock;

pub struct SampledImage {
    device: Arc<Device>,
    image: VkHandle<vk::Image>,
    frame_lock: FrameLock,
    dimensions: Dimensions,
    mipmaps_count: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    samples: u32,
    layout: vk::ImageLayout,
}

impl SampledImage {
    pub fn uninitialized(
        device: &Arc<Device>,
        dimensions: Dimensions,
        format: vk::Format,
        mipmaps_count: MipmapsCount,
        samples: u32,
    ) -> SampledImage {
        let image = UnboundImage::new(
            device,
            dimensions,
            mipmaps_count,
            samples,
            format,
            vk::IMAGE_USAGE_SAMPLED_BIT | vk::IMAGE_USAGE_TRANSFER_DST_BIT,
            vk::ImageLayout::Undefined,
            SharingMode::Exclusive,
        );

        // determine size of image memory and staging buffer
        // allocate staging buffer
        // allocate image memory

        let memory_requirements = image.memory_requirements();

        unimplemented!()
    }
}
