//! Device creation
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;
use std::u32;

use ash;
use ash::extensions;
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0, V1_0};
use ash::vk;
use config::Config;
use sid_vec::{Id, IdVec};
use winit::Window;

use instance::{Instance, VkInstance1};
use surface::Surface;
use sync::{FrameSync, SyncGroup};

pub type VkDevice1 = ash::Device<V1_0>;
pub struct QueueTag;
pub type QueueId = Id<QueueTag>;

mod physical_device;
mod queue;

// queues: different queue families, each queue family has different properties
// resources are shared between different queue families, not queues
pub struct Queue {
    family: u32,
    queue: vk::Queue,
    capabilities: vk::QueueFlags,
}

pub struct DeviceExtensionPointers {
    vk_khr_swapchain: extensions::Swapchain,
}

pub struct Queues {
    present: (u32, vk::Queue),
    transfer: (u32, vk::Queue),
    graphics: (u32, vk::Queue),
    compute: (u32, vk::Queue),
}

/// Vulkan device.
pub struct Device {
    instance: Arc<Instance>,
    pointers: VkDevice1,
    extension_pointers: DeviceExtensionPointers,
    physical_device: vk::PhysicalDevice,
    queues: Queues,
    max_frames_in_flight: u32,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    //frame_sync: FrameSync,
}

impl Device {
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn pointers(&self) -> &VkDevice1 {
        &self.pointers
    }

    pub fn new(
        instance: &Arc<Instance>,
        config: &Config,
        target_surface: Option<&Surface>,
    ) -> Arc<Device> {
        let max_frames_in_flight = cfg.get::<u32>("gfx.renderer.max_frames_in_flight").unwrap();

        // select physical device
        let physical_device_selection =
            physical_device::select_physical_device(instance, target_surface)
                .expect("unable to find a suitable physical device");

        // select the queue families to create
        let queue_config = queue::create_queue_configuration(
            instance,
            physical_device_selection.physical_device,
            &physical_device_selection.queue_family_properties,
            target_surface,
        );

        // setup queue create infos
        let num_queue_families = physical_device_selection.queue_family_properties.len();
        let mut queue_priorities = Vec::new();
        for i in 0..num_queue_families {
            if queue_config.num_queues[i] > 0 {
                // FIXME no priorities for now
                queue_priorities.push(vec![1.0f32; num_queues[i] as usize]);
            }
        }

        let mut queue_create_info = Vec::new();
        for i in 0..num_queue_families {
            if num_queues[i] > 0 {
                queue_create_info.push(vk::DeviceQueueCreateInfo {
                    s_type: vk::StructureType::DeviceQueueCreateInfo,
                    p_next: ptr::null(),
                    flags: vk::DeviceQueueCreateFlags::empty(),
                    queue_family_index: i as u32,
                    queue_count: queue_config.num_queues[i],
                    p_queue_priorities: queue_priorities[i].as_ptr(),
                });
            }
        }

        let device_extension_names_raw = [extensions::Swapchain::name().as_ptr()];

        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DeviceCreateInfo,
            p_next: ptr::null(),
            flags: Default::default(),
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            enabled_extension_count: device_extension_names_raw.len() as u32,
            pp_enabled_extension_names: device_extension_names_raw.as_ptr(),
            p_enabled_features: &features,
        };

        let vkd = unsafe {
            instance
                .pointers()
                .create_device(selected_physical_device, &device_create_info, None)
                .expect("unable to create device")
        };

        let queues = unsafe {
            Queues {
                present: (
                    queue_config.present.0,
                    vkd.get_device_queue(queue_config.present.0, queue_config.present.1),
                ),
                transfer: (
                    queue_config.transfer.0,
                    vkd.get_device_queue(queue_config.transfer.0, queue_config.transfer.1),
                ),
                graphics: (
                    queue_config.graphics.0,
                    vkd.get_device_queue(queue_config.graphics.0, queue_config.graphics.1),
                ),
                compute: (
                    queue_config.compute.0,
                    vkd.get_device_queue(queue_config.compute.0, queue_config.compute.1),
                ),
            }
        };

        let extension_pointers = DeviceExtensionPointers {
            vk_khr_swapchain: extensions::Swapchain::new(instance, vkd)
                .expect("unable to load swapchain extension"),
        };

        let image_available = {
            let info = vk::SemaphoreCreateInfo {
                s_type: vk::StructureType::SemaphoreCreateInfo,
                p_next: ptr::null(),
                flags: vk::SemaphoreCreateFlags::empty(),
            };
            unsafe { vkd.create_semaphore(&info, None).unwrap() }
        };

        let image_available = {
            let info = vk::SemaphoreCreateInfo {
                s_type: vk::StructureType::SemaphoreCreateInfo,
                p_next: ptr::null(),
                flags: vk::SemaphoreCreateFlags::empty(),
            };
            unsafe { vkd.create_semaphore(&info, None).unwrap() }
        };

        Arc::new(Device {
            physical_device: physical_device_selection.physical_device,
            queues,
            pointers: vkd,
            instance: instance.clone(),
            extension_pointers,
            image_available,
            render_finished,
            max_frames_in_flight,
            //frame_sync: FrameSync::new(),
        })
    }
}

//--------------------------------------------------------------------------------------------------
// SYNC GROUPS

/*
/// A sync group regroups resources that should wait on (one or more) semaphores before being used again.
/// Resources are assigned SyncGroupIds when they are submitted to the pipeline.
pub(crate) struct SyncGroup {}
*/

//--------------------------------------------------------------------------------------------------
// CONTEXT

/// A frame number. Represents a point in time that corresponds to the completion
/// of a frame.
/// E.g. a value of 42 represents the instant of completion of frame 42.
/// The frames start at 1. The value 0 is reserved.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FrameNumber(pub(crate) u64);

/*
/// Reinitializes a presentation object.
pub fn reset_presentation(&mut self, presentation: &mut Presentation) {
    // TODO: wait for all commands complete before deleting the resources associated with the presentation.
    unsafe {
        presentation.recreate_swapchain(
            &self.vke,
            &self.vki,
            &self.vkd,
            &self.surface_loader,
            &self.swapchain_loader,
            self.physical_device,
        );
    }
}

/// Acquires a presentation image.
pub fn acquire_presentation_image<'a>(&mut self, presentation: &'a Presentation) -> &'a Image {
    let next_image = unsafe {
        self.swapchain_loader
            .acquire_next_image_khr(
                presentation.swapchain,
                u64::max_value(),
                self.image_available,
                vk::Fence::null(),
            ).unwrap()
    };
    let img = &presentation.images[next_image as usize];
    img
}

/// Returns the default memory allocator.
pub fn default_allocator(&self) -> &Allocator {
    &self.allocator
}

pub fn vk_instance(&self) -> &VkInstance1 {
    &self.vki
}

pub fn vk_device(&self) -> &VkDevice1 {
    &self.vkd
}
}*/
