use super::*;
use sid_vec::{Id, IdVec};

mod buffer;
mod image;

pub use self::buffer::*;
pub use self::image::*;

use ash::vk;
use crate::buffer::{Buffer, BufferDescription};
use crate::image::{Dimensions, Image, ImageDescription};

pub trait Resource {
    fn name(&self) -> &str;
    fn is_transient(&self) -> bool;
    fn is_allocated(&self) -> bool;
}

pub trait ImageResource: Resource + ImageDescription {
    fn set_usage(&mut self, usage: vk::ImageUsageFlags) -> bool;
    fn initial_layout(&self) -> vk::ImageLayout;
    /// The associated swapchain, if the image is part of a swapchain.
    fn swapchain(&self) -> Option<vk::SwapchainKHR>;
    /// The associated swapchain index, if the image is part of a swapchain.
    fn swapchain_index(&self) -> Option<u32>;
}

pub trait BufferResource: Resource + BufferDescription {}
