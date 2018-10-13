use ash::vk;

use crate::device::Device;
use crate::image::{MipmapsCount, Dimensions};

pub trait Image
{
    fn device(&self) -> &Device;
    fn dimensions(&self) -> &Dimensions;
    fn format(&self) -> &vk::Format;

}