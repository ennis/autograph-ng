use ash::vk;

use crate::device::{Device, DeviceBoundObject, FrameNumber, FrameSynchronizedObject};
use crate::image::{Dimensions, MipmapsCount};
use crate::sync::{SignalSemaphore, WaitSemaphore};

pub unsafe trait ImageProxy {
    /// Returns the image. Only valid for the duration of the frame in which the image proxy was created.
    fn image(&self) -> vk::Image;
    /// Returns the layout at the time of acquisition.
    fn initial_layout(&self) -> vk::ImageLayout;
}

/// Characteristics of an image.
pub trait ImageDescription {
    fn dimensions(&self) -> Dimensions;
    fn mipmaps_count(&self) -> u32;
    fn samples(&self) -> u32;
    fn format(&self) -> vk::Format;
    fn usage(&self) -> vk::ImageUsageFlags;
}

/// Trait implemented by types that wrap around a vulkan image.
pub trait Image: FrameSynchronizedObject {
    fn device(&self) -> &Device;

    /// Expected layout of the image once all operations affecting the image
    /// have finished.
    fn layout(&self) -> vk::ImageLayout;

    fn handle(&self) -> vk::Image;
}

pub trait ImageTag {
    const FORMAT: Option<vk::Format>;
    const USAGE: Option<vk::ImageUsageFlags>;
}

pub trait TaggedImage {
    type Tag: ImageTag;
}

pub struct AnyImageTag {}

impl ImageTag for AnyImageTag {
    const FORMAT: Option<vk::Format> = None;
    const USAGE: Option<vk::ImageUsageFlags> = None;
}

/*
// Marker traits that define image capabilities.

pub trait TransferSourceCapability: ImageRequirements
{}

pub trait TransferDestinationCapability: ImageRequirements
{}

pub trait SampleCapability: ImageRequirements
{}

pub trait StorageCapability: ImageRequirements
{}

pub trait ColorAttachmentCapability: ImageRequirements
{}

pub trait DepthAttachmentCapability: ImageRequirements
{}

pub trait AttachmentCapability: ImageRequirements
{}

pub trait TransientAttachmentCapability: ImageRequirements
{}

macro_rules! define_image_type {
    (

    ) => {};
}

pub trait ImageRequirements
{
    /// What we know statically about the format
    const Format: Option<Format>;
}

pub trait ImageTag<T: ImageRequirements> {

}

// T: Image + TransferSourceCapability + SampleCapability
// T: Image + ImageTag<U> where U: SampleCapability + AttachmentCapability
// Swapchain::<GeneralImageUsage>::new() -> impl Image + ImageUsageTag<GeneralImageUsage>
//
//

*/
