use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::sync::{Arc, Mutex, Weak};
use std::u32;
use ash;
use ash::extensions;
use ash::vk;
use config::Config;
use sid_vec::{Id, IdVec};
use winit::Window;

mod instance;
mod renderer;
mod memory;
mod handle;
mod physical_device;
mod queue;

use crate::renderer::*;
use crate::renderer::vk::instance::create_instance;
use crate::renderer::vk::memory::{MemoryBlock, MemoryPool};
use crate::renderer::vk::surface::Surface;

pub use self::renderer::VulkanRenderer;

pub struct QueueTag;
pub type QueueId = Id<QueueTag>;

// queues: different queue families, each queue family has different properties
// resources are shared between different queue families, not queues
pub struct Queue {
    family: u32,
    queue: vk::Queue,
    capabilities: vk::QueueFlags,
}

pub struct Queues {
    present: (u32, vk::Queue),
    transfer: (u32, vk::Queue),
    graphics: (u32, vk::Queue),
    compute: (u32, vk::Queue),
}

/// Vulkan device.
pub struct VulkanRenderer {
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    vk_ext_debug_report: ash::extensions::DebugReport,
    vk_khr_surface: ash::extensions::Surface,
    vk_khr_swapchain: ash::extensions::Swapchain,
    physical_device: vk::PhysicalDevice,
    queues: Queues,
    max_frames_in_flight: u32,
    default_pool_block_size: u64,
    default_pool: MemoryPool,
    //frame_fence: FrameFence,
    // allocated objects:
    // - handle
    // - minimum metadata
    // - pending uses in frame
    // - sync across frames
    // - marked for deletion
}

//--------------------------------------------------------------------------------------------------
impl VulkanRenderer {

    pub fn extension_pointers(&self) -> &DeviceExtensionPointers {
        &self.extension_pointers
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    pub fn max_frames_in_flight(&self) -> u32 {
        self.max_frames_in_flight
    }

    /*pub fn concurrent_across_queue_families(&self) -> SharingMode {
        let mut queue_families = [
            self.queues.present.0,
            self.queues.transfer.0,
            self.queues.graphics.0,
            self.queues.compute.0,
        ]
            .to_vec();
        queue_families.sort();
        queue_families.dedup();
        SharingMode::Concurrent(queue_families)
    }*/

    pub fn is_frame_retired(&self, frame_number: FrameNumber) -> bool {
        self.frame_fence.last_retired_frame() >= frame_number
    }

    pub fn last_retired_frame(&self) -> FrameNumber {
        self.frame_fence.last_retired_frame()
    }

    pub fn current_frame(&self) -> FrameNumber {
        self.frame_fence.current_frame()
    }

    pub fn default_graphics_queue(&self) -> (u32, vk::Queue) {
        self.queues.graphics
    }

    pub fn default_compute_queue(&self) -> (u32, vk::Queue) {
        self.queues.compute
    }

    pub fn default_transfer_queue(&self) -> (u32, vk::Queue) {
        self.queues.transfer
    }

    pub fn default_present_queue(&self) -> (u32, vk::Queue) {
        self.queues.present
    }

    pub fn create_semaphore(&self) -> (SignalSemaphore, WaitSemaphore) {
        unimplemented!()
    }
}


//--------------------------------------------------------------------------------------------------
impl Renderer for VulkanRenderer
{
    fn create_swapchain(&self) -> SwapchainHandle {
        unimplemented!()
    }

    fn create_image(&self, dimensions: Dimensions, mipcount: MipmapsCount, samples: u32, format: Format, usage: ImageUsageFlags) -> ImageHandle {
        unimplemented!()
    }

    fn upload_transient(&self, data: &[u8]) -> BufferHandle {
        unimplemented!()
    }

    fn destroy_image(&self, image: ImageHandle) {
        unimplemented!()
    }

    fn create_buffer(&self, size: u64) -> BufferHandle {
        unimplemented!()
    }

    fn destroy_buffer(&self, buffer: BufferHandle) {
        unimplemented!()
    }

    fn submit_command_buffer(&self, cmdbuf: _) {
        unimplemented!()
    }

    fn end_frame(&self) {
        unimplemented!()
    }
}

