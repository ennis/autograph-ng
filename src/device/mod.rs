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

/// Vulkan device.
pub struct Device {
    instance: Arc<Instance>,
    pointers: VkDevice1,
    extension_pointers: DeviceExtensionPointers,
    physical_device: vk::PhysicalDevice,
    queues: IdVec<QueueId, Queue>,
    preferred_transfer_queue: QueueId,
    present_queue: QueueId,
    max_frames_in_flight: u32,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    frame_sync: FrameSync,
}

impl Device {
    pub fn new(
        instance: &Arc<Instance>,
        config: &Config,
        target_surface: Option<&Surface>,
    ) -> Arc<Device> {
        let physical_device_selection =
            physical_device::select_physical_device(instance, target_surface);

        // select the queue families to create
        // heuristic: select a queue for async-compute, a queue for std graphics, a queue for transfer, and a queue family supporting presentation

        // create the device and queues
        let selected_queue_family_index = selected_queue_family_index.unwrap();
        // create the queue and the device
        let priorities = [1.0];
        let queue_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DeviceQueueCreateInfo,
            p_next: ptr::null(),
            flags: Default::default(),
            queue_family_index: selected_queue_family_index,
            p_queue_priorities: priorities.as_ptr(),
            queue_count: priorities.len() as u32,
        };

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

        let vkd = vki
            .create_device(selected_physical_device, &device_create_info, None)
            .expect("Unable to create device");
        let present_queue = vkd.get_device_queue(selected_queue_family_index as u32, 0);
        // let's assume that the queue is also a graphics queue...
        DeviceAndQueues {
            graphics_queue: present_queue,
            present_queue: present_queue,
            graphics_queue_family_index: selected_queue_family_index,
            present_queue_family_index: selected_queue_family_index,
            vkd,
            physical_device: selected_physical_device,
        }
    }
}

/// Helper function to create a vulkan device and queues that are compatible with
/// the specified surface (if any).
unsafe fn create_device_and_queues(
    vke: &VkEntry1,
    vki: &VkInstance1,
    surface_loader: &extensions::Surface,
    surface: Option<vk::SurfaceKHR>,
    config: &Config,
) -> DeviceAndQueues {
    let physical_devices = vki
        .enumerate_physical_devices()
        .expect("Physical device error");

    // The selected physical device.
    // TODO prefer discrete graphics?
    let mut selected_physical_device: Option<vk::PhysicalDevice> = None;
    // The selected queue family index. Must support graphics.
    let mut selected_queue_family_index: Option<u32> = None;

    'outer: for physical_device in physical_devices.iter() {
        // Print physical device name
        let dev_info = vki.get_physical_device_properties(*physical_device);
        let dev_name = unsafe {
            CStr::from_ptr(&dev_info.device_name[0])
                .to_owned()
                .into_string()
                .unwrap()
        };
        info!("Physical device: {}", dev_name);

        let queue_family_props = vki.get_physical_device_queue_family_properties(*physical_device);
        for (queue_family_index, ref queue_family_info) in queue_family_props.iter().enumerate() {
            info!(
                "Queue family #{}: {:?}",
                queue_family_index, queue_family_info
            );
            // does the queue supports graphics?
            let supports_graphics = queue_family_info.queue_flags.subset(vk::QUEUE_GRAPHICS_BIT);

            // is the queue compatible with the surface we just created?
            let supports_surface = if let Some(surface) = surface {
                surface_loader.get_physical_device_surface_support_khr(
                    *physical_device,
                    queue_family_index as u32,
                    surface,
                )
            } else {
                true
            };
            if supports_graphics && supports_surface {
                // OK, choose this queue and physical device.
                selected_physical_device = Some(*physical_device);
                selected_queue_family_index = Some(queue_family_index as u32);
                break 'outer;
            }
        }
    }

    if let Some(selected_physical_device) = selected_physical_device {

    } else {
        panic!("Unable to find a suitable physical device and queue family");
    }
}

/// Helper function to create a command pool for a given queue family.
unsafe fn create_command_pool_for_queue(
    vkd: &VkDevice1,
    queue_family_index: u32,
) -> vk::CommandPool {
    let command_pool_create_info = vk::CommandPoolCreateInfo {
        s_type: vk::StructureType::CommandPoolCreateInfo,
        p_next: ptr::null(),
        flags: vk::COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
        queue_family_index,
    };

    vkd.create_command_pool(&command_pool_create_info, None)
        .unwrap()
}

pub(crate) unsafe fn create_semaphore(vkd: &VkDevice1) -> vk::Semaphore {
    let info = vk::SemaphoreCreateInfo {
        s_type: vk::StructureType::SemaphoreCreateInfo,
        flags: vk::SemaphoreCreateFlags::default(),
        p_next: ptr::null(),
    };
    vkd.create_semaphore(&info, None)
        .expect("failed to create semaphore")
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

/// Main graphics context.
/// Handles allocation of persistent resources.
pub struct Context {
    pub(crate) vke: VkEntry1,
    pub(crate) vki: VkInstance1,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) vkd: VkDevice1,
    pub(crate) graphics_queue_family_index: u32,
    pub(crate) present_queue_family_index: u32,
    pub(crate) graphics_queue: vk::Queue,
    pub(crate) present_queue: vk::Queue,
    pub(crate) graphics_queue_command_pool: vk::CommandPool,
    pub(crate) present_queue_command_pool: vk::CommandPool,
    pub(crate) surface_loader: extensions::Surface,
    pub(crate) swapchain_loader: extensions::Swapchain,
    pub(crate) max_frames_in_flight: u32,
    pub(crate) image_available: vk::Semaphore,
    pub(crate) render_finished: vk::Semaphore,
    pub(crate) allocator: Allocator,
    pub(crate) frame_sync: FrameSync,
}

impl Context {
    /// Creates a new context and associated `Presentation` objects.
    pub fn new(
        presentation_targets: &[&PresentationTarget],
        cfg: &Config,
    ) -> (Context, Vec<Presentation>) {
        // Load settings
        let initial_upload_buffer_size =
            cfg.get::<usize>("gfx.default_upload_buffer_size").unwrap();
        let max_frames_in_flight = cfg.get::<usize>("gfx.max_frames_in_flight").unwrap();
        let vk_instance_extensions = cfg
            .get::<Vec<String>>("gfx.vulkan.instance_extensions")
            .unwrap();
        let vk_layers = cfg.get::<Vec<String>>("gfx.vulkan.layers").unwrap();
        let vk_default_alloc_block_size = cfg
            .get::<u64>("gfx.vulkan.default_alloc_block_size")
            .unwrap();

        // TODO split up the unsafe block
        unsafe {
            let vke = VkEntry1::new().unwrap();
            let app_raw_name = CStr::from_bytes_with_nul(b"Autograph/GFX\0")
                .unwrap()
                .as_ptr();

            let mut layer_names = Vec::new();
            layer_names.push(CString::new("VK_LAYER_LUNARG_standard_validation").unwrap());
            layer_names.extend(
                vk_layers
                    .iter()
                    .map(|name| CString::new(name.clone()).unwrap()),
            );
            let layers_names_raw: Vec<*const c_char> = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            let extension_names_raw = extension_names();
            let appinfo = vk::ApplicationInfo {
                p_application_name: app_raw_name,
                s_type: vk::StructureType::ApplicationInfo,
                p_next: ptr::null(),
                application_version: 0,
                p_engine_name: app_raw_name,
                engine_version: 0,
                api_version: vk_make_version!(1, 0, 36),
            };
            let create_info = vk::InstanceCreateInfo {
                s_type: vk::StructureType::InstanceCreateInfo,
                p_next: ptr::null(),
                flags: Default::default(),
                p_application_info: &appinfo,
                pp_enabled_layer_names: layers_names_raw.as_ptr(),
                enabled_layer_count: layers_names_raw.len() as u32,
                pp_enabled_extension_names: extension_names_raw.as_ptr(),
                enabled_extension_count: extension_names_raw.len() as u32,
            };
            let vki: VkInstance1 = vke
                .create_instance(&create_info, None)
                .expect("Instance creation error");

            let debug_info = vk::DebugReportCallbackCreateInfoEXT {
                s_type: vk::StructureType::DebugReportCallbackCreateInfoExt,
                p_next: ptr::null(),
                flags: vk::DEBUG_REPORT_ERROR_BIT_EXT
                    | vk::DEBUG_REPORT_WARNING_BIT_EXT
                    | vk::DEBUG_REPORT_PERFORMANCE_WARNING_BIT_EXT
                    | vk::DEBUG_REPORT_DEBUG_BIT_EXT
                    | vk::DEBUG_REPORT_INFORMATION_BIT_EXT,
                pfn_callback: vulkan_debug_callback,
                p_user_data: ptr::null_mut(),
            };

            //---------------------------------
            // set debug report callback
            let debug_report_loader =
                extensions::DebugReport::new(&vke, &vki).expect("Unable to load debug report");
            let debug_call_back = debug_report_loader
                .create_debug_report_callback_ext(&debug_info, None)
                .unwrap();

            //---------------------------------
            // we have an instance, now create a device that best fits the presentation target
            assert!(
                presentation_targets.len() <= 1,
                "Cannot yet specify more than one presentation target"
            );

            // create surfaces for each presentation target
            let mut surfaces = Vec::new();
            for t in presentation_targets {
                let surf = match t {
                    PresentationTarget::Window(ref window) => {
                        create_surface(&vke, &vki, window).expect("Unable to create a surface")
                    }
                    _ => {
                        panic!("Cannot create a surface without a window");
                    }
                };
                surfaces.push(surf)
            }

            // create device and queues
            let surface_loader =
                extensions::Surface::new(&vke, &vki).expect("Unable to load surface extension");
            let device_and_queues = create_device_and_queues(
                &vke,
                &vki,
                &surface_loader,
                surfaces.first().cloned(),
                cfg,
            );

            //---------------------------------
            // create swapchains for each initial presentation target
            let swapchain_loader = extensions::Swapchain::new(&vki, &device_and_queues.vkd)
                .expect("Unable to load swapchain extension");

            let mut presentations = Vec::new();

            for (i, t) in presentation_targets.iter().enumerate() {
                let surface = surfaces[i];
                match t {
                    PresentationTarget::Window(ref window) => {
                        let hidpi_factor = window.get_hidpi_factor();
                        let (window_width, window_height): (u32, u32) = window
                            .get_inner_size()
                            .unwrap()
                            .to_physical(hidpi_factor)
                            .into();
                        // FIXME: should put swapchain parameters in PresentationTarget
                        let (swapchain, swapchain_create_info) = create_swapchain(
                            &vke,
                            &vki,
                            &device_and_queues.vkd,
                            &surface_loader,
                            &swapchain_loader,
                            device_and_queues.physical_device,
                            window_width,
                            window_height,
                            surface,
                        );
                        let mut swapchain_images = swapchain_loader
                            .get_swapchain_images_khr(swapchain)
                            .unwrap();
                        let swapchain_images = swapchain_images
                            .drain(..)
                            .enumerate()
                            .map(|(i, img)| {
                                Image::new_swapchain_image(
                                    "presentation image",
                                    &swapchain_create_info,
                                    OwnedHandle::new(img),
                                    i as u32,
                                )
                            }).collect::<Vec<_>>();

                        presentations.push(Presentation {
                            target: (*t).clone(),
                            surface,
                            swapchain,
                            images: swapchain_images,
                        });
                    }
                    _ => panic!("Cannot create a swapchain without a window"),
                }
            }

            //---------------------------------
            // create command pools
            let present_pool = create_command_pool_for_queue(
                &device_and_queues.vkd,
                device_and_queues.present_queue_family_index,
            );
            let graphics_pool = create_command_pool_for_queue(
                &device_and_queues.vkd,
                device_and_queues.graphics_queue_family_index,
            );

            //---------------------------------
            // create semaphores to sync between the draw and present queues
            let image_available = create_semaphore(&device_and_queues.vkd);
            let render_finished = create_semaphore(&device_and_queues.vkd);
            let allocator = Allocator::new(
                &vki,
                &device_and_queues.vkd,
                device_and_queues.physical_device,
                vk_default_alloc_block_size,
            );

            (
                Context {
                    vke,
                    vki,
                    physical_device: device_and_queues.physical_device,
                    vkd: device_and_queues.vkd,
                    graphics_queue_family_index: device_and_queues.graphics_queue_family_index,
                    present_queue_family_index: device_and_queues.present_queue_family_index,
                    graphics_queue: device_and_queues.graphics_queue,
                    present_queue: device_and_queues.present_queue,
                    graphics_queue_command_pool: graphics_pool,
                    present_queue_command_pool: present_pool,
                    surface_loader,
                    swapchain_loader,
                    max_frames_in_flight: max_frames_in_flight as u32,
                    image_available,
                    render_finished,
                    allocator,
                    frame_sync: FrameSync::new(FrameNumber(1), max_frames_in_flight as u32),
                },
                presentations,
            )
        }
    }

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
}

// persistent image creation
// - must know in advance which queue families will use them.
