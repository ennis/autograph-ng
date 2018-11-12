use std::cell::Cell;
use std::cmp::max;
use std::ptr;
use std::sync::Arc;

use ash::vk;

use crate::device::{Device, DeviceBoundObject, FrameNumber, FrameSynchronizedObject};
use crate::handle::VkHandle;
use crate::image::traits::*;
use crate::image::Dimensions;
use crate::swapchain::Swapchain;
use crate::sync::FrameLock;

/// Generic wrapper around a vulkan image.
pub struct GenericImage {
    device: Arc<Device>,
    image: VkHandle<vk::Image>,
    frame_lock: FrameLock,
    layout: vk::ImageLayout,
}

impl DeviceBoundObject for GenericImage {
    fn device(&self) -> &Device {
        &self.device
    }
}

pub struct GenericImageProxy(vk::Image, vk::ImageLayout);

unsafe impl ImageProxy for GenericImageProxy {
    fn image(&self) -> vk::Image {
        self.0
    }

    fn initial_layout(&self) -> vk::ImageLayout {
        self.1
    }
}

unsafe impl FrameSynchronizedObject for GenericImage {
    type Proxy = GenericImageProxy;

    unsafe fn lock(
        &self,
        frame_number: FrameNumber,
    ) -> (
        GenericImageProxy,
        Option<vk::Semaphore>,
        Option<vk::Semaphore>,
    ) {
        let (entry, exit) = self.frame_lock.lock(frame_number);
        (
            GenericImageProxy(self.image.get(), self.layout),
            entry,
            Some(exit),
        )
    }
}

impl Image for GenericImage {
    fn device(&self) -> &Device {
        &self.device
    }

    fn layout(&self) -> vk::ImageLayout {
        unimplemented!()
    }

    fn handle(&self) -> vk::Image {
        self.image.get()
    }
}

impl Drop for GenericImage {
    fn drop(&mut self) {
        if !self.device.is_frame_retired(self.frame_lock.locked_until()) {
            panic!("image may have been dropped while still in use")
        }
    }
}
