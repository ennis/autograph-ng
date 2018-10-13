use ash::vk;

use crate::device::Device;
use crate::image::{Dimensions, MipmapsCount};

/// Characteristics of an image.
pub trait ImageDescription {
    fn dimensions(&self) -> Dimensions;
    fn mipmaps_count(&self) -> u32;
    fn samples(&self) -> u32;
    fn format(&self) -> vk::Format;
    fn usage(&self) -> vk::ImageUsageFlags;
}

/// Trait implemented by types that wrap around a vulkan image.
pub trait Image: ImageDescription {
    fn device(&self) -> &Device;

    /// Expected layout of the image once all operations affecting the image
    /// have finished.
    fn layout(&self) -> vk::ImageLayout;
}
