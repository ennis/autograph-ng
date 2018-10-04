use ash::vk;

use super::Dimensions;

pub trait ImageDescription {
    fn dimensions(&self) -> Dimensions;
    fn mipmaps_count(&self) -> u32;
    fn format(&self) -> vk::Format;
    fn usage(&self) -> vk::ImageUsageFlags;
}

