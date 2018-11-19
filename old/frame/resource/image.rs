use std::cell::Cell;
use std::ops::Deref;
use std::sync::Arc;

use crate::frame::graph::TaskId;
use crate::frame::resource::{ImageResource, Resource};
use crate::frame::tasks::TaskOutput;
use crate::frame::LifetimeId;
use crate::sync::{SignalSemaphore, WaitSemaphore};

use ash::vk;
use sid_vec::{Id, IdVec};

use crate::device::{FrameNumber, FrameSynchronizedObject};
use crate::image::{Dimensions, GenericImage, Image, ImageDescription, ImageProxy};
use crate::swapchain::{Swapchain, SwapchainImageProxy};

//--------------------------------------------------------------------------------------------------
pub struct ImportedImageResource {
    image: vk::Image,
    format: vk::Format,
    dimensions: Dimensions,
    usage: vk::ImageUsageFlags,
    samples: u32,
    mipmaps: u32,
    initial_layout: vk::ImageLayout,
    entry_semaphore: Option<vk::Semaphore>,
    exit_semaphore: Option<vk::Semaphore>,
}

impl Resource for ImportedImageResource {
    fn name(&self) -> &str {
        "unnamed image"
    }

    fn is_transient(&self) -> bool {
        false
    }

    fn is_allocated(&self) -> bool {
        true
    }
}

impl ImportedImageResource {
    pub fn new<I, IP, ID>(image: &I, frame: FrameNumber) -> ImportedImageResource
    where
        IP: ImageProxy + 'static,
        ID: ImageDescription,
        I: FrameSynchronizedObject<Proxy = IP> + Deref<Target = ID>,
    {
        let (proxy, entry_semaphore, exit_semaphore) = unsafe { image.lock(frame) };

        ImportedImageResource {
            image: proxy.image(),
            format: image.format(),
            dimensions: image.dimensions(),
            usage: image.usage(),
            samples: image.samples(),
            mipmaps: image.mipmaps_count(),
            initial_layout: proxy.initial_layout(),
            entry_semaphore,
            exit_semaphore,
        }
    }
}

impl ImageDescription for ImportedImageResource {
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn mipmaps_count(&self) -> u32 {
        self.mipmaps
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

impl ImageResource for ImportedImageResource {
    fn set_usage(&mut self, usage: vk::ImageUsageFlags) -> bool {
        self.usage.subset(usage)
    }

    fn initial_layout(&self) -> vk::ImageLayout {
        self.initial_layout
    }

    fn swapchain(&self) -> Option<vk::SwapchainKHR> {
        None
    }

    fn swapchain_index(&self) -> Option<u32> {
        None
    }
}

//--------------------------------------------------------------------------------------------------
pub struct SwapchainImageResource {
    proxy: SwapchainImageProxy,
    format: vk::Format,
    dimensions: Dimensions,
    usage: vk::ImageUsageFlags,
    samples: u32,
    mipmaps: u32,
    image_available: vk::Semaphore,
}

impl SwapchainImageResource {
    pub fn new(swapchain: &Arc<Swapchain>, frame_number: FrameNumber) -> SwapchainImageResource {
        let (proxy, image_available, _) = unsafe { swapchain.lock(frame_number) };

        SwapchainImageResource {
            proxy,
            format: swapchain.format(),
            dimensions: swapchain.dimensions(),
            usage: swapchain.usage(),
            samples: 0,
            mipmaps: 0,
            image_available: image_available.unwrap(),
        }
    }
}

impl Resource for SwapchainImageResource {
    fn name(&self) -> &str {
        "unnamed swapchain image"
    }

    fn is_transient(&self) -> bool {
        false
    }

    fn is_allocated(&self) -> bool {
        true
    }
}

impl ImageResource for SwapchainImageResource {
    fn set_usage(&mut self, usage: vk::ImageUsageFlags) -> bool {
        self.usage.subset(usage)
    }

    fn initial_layout(&self) -> vk::ImageLayout {
        vk::ImageLayout::Undefined
    }

    fn swapchain(&self) -> Option<vk::SwapchainKHR> {
        Some(self.proxy.swapchain())
    }

    fn swapchain_index(&self) -> Option<u32> {
        Some(self.proxy.swapchain_index())
    }
}

impl ImageDescription for SwapchainImageResource {
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn mipmaps_count(&self) -> u32 {
        self.mipmaps
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

//--------------------------------------------------------------------------------------------------
struct TransientImageResource {
    format: vk::Format,
    dimensions: Dimensions,
    usage: vk::ImageUsageFlags,
    samples: u32,
    mipmaps: u32,
    initial_layout: vk::ImageLayout,
    image: Option<GenericImage>,
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

impl ImageDescription for TransientImageResource {
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn mipmaps_count(&self) -> u32 {
        self.mipmaps
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

impl ImageResource for TransientImageResource {
    fn set_usage(&mut self, usage: vk::ImageUsageFlags) -> bool {
        self.usage |= usage;
        true
    }

    fn initial_layout(&self) -> vk::ImageLayout {
        self.initial_layout
    }

    fn swapchain(&self) -> Option<vk::SwapchainKHR> {
        None
    }

    fn swapchain_index(&self) -> Option<u32> {
        None
    }
}

//--------------------------------------------------------------------------------------------------
pub struct ImageTag;
/// Identifies an image in the frame resource table.
pub type ImageId = Id<ImageTag, u32>;
