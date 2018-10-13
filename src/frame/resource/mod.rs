use super::*;
use sid_vec::{Id, IdVec};

pub mod buffer;
pub mod image;

pub use self::buffer::*;
pub use self::image::*;

use ash::vk;
use crate::image::Dimensions;

pub trait Resource {
    fn name(&self) -> &str;
    fn is_transient(&self) -> bool;
    fn is_allocated(&self) -> bool;
}

pub trait ImageResource: Resource {
    fn dimensions(&self) -> Dimensions;
    fn format(&self) -> vk::Format;
    fn samples(&self) -> u32;
    fn set_usage(&mut self, usage: vk::ImageUsageFlags) -> bool;
}

pub trait BufferResource: Resource {
    fn byte_size(&self) -> u64;
}
