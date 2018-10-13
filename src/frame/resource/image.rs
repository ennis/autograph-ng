use frame::graph::TaskId;
use frame::resource::{ImageResource, Resource};
use frame::tasks::TaskOutput;
use frame::LifetimeId;

use ash::vk;
use sid_vec::{Id, IdVec};

use crate::image::Dimensions;
//use crate::image::unbound::UnboundImage;

//--------------------------------------------------------------------------------------------------
struct TransientImageResource {
    format: vk::Format,
    dimensions: Dimensions,
    usage: vk::ImageUsageFlags,
    samples: u32,
    mipmaps: u32,
    image: Option<UnsafeImage>,
}

impl Resource for TransientImageResource {
    fn name(&self) -> &str {
        "unnamed image"
    }

    fn is_transient(&self) -> bool {
        true
    }

    fn is_allocated(&self) -> bool {
        self.image.is_some()
    }
}

impl ImageResource for TransientImageResource {
    fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    fn format(&self) -> vk::Format {
        self.format
    }

    fn samples(&mut self) -> u32 {
        self.samples
    }

    fn set_usage(&mut self, usage: vk::ImageUsageFlags) -> bool {
        self.usage |= usage;
        true
    }
}

//--------------------------------------------------------------------------------------------------
pub struct ImageTag;
/// Identifies an image in the frame resource table.
pub type ImageId = Id<ImageTag, u32>;
