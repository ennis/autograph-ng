use config::Config;
use winit::Window;
use ash;
use ash::vk;
use std::ptr;
use std::sync::Mutex;

use crate::renderer::vk::VulkanRenderer;
use crate::renderer::vk::memory::MemoryPool;
use crate::renderer::vk::queue::create_queue_configuration;
use crate::renderer::vk::physical_device::select_physical_device;
use crate::renderer::vk::instance::{create_instance, InstanceAndExtensions};


impl VulkanRenderer
{
    pub fn new_inner(
        cfg: &Config,
        window: &Window
    ) -> VulkanRenderer
    {
        // create instance
        let InstanceAndExtensions {
            entry,
            instance,
            vk_ext_debug_report,
            vk_khr_surface,
        } = create_instance(cfg);

        let max_frames_in_flight = cfg.get::<u32>("gfx.max_frames_in_flight").unwrap();
        let default_alloc_block_size = cfg.get::<u64>("gfx.default_alloc_block_size").unwrap();

        // select physical device
        let physical_device_selection =
            select_physical_device(instance, target_surface)
                .expect("unable to find a suitable physical device");

        // select the queue families to create
        let queue_config = create_queue_configuration(
            &instance,
            &vk_khr_surface,
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
                queue_priorities.push(vec![1.0f32; queue_config.num_queues[i] as usize]);
            }
        }

        let mut queue_create_info = Vec::new();
        for i in 0..num_queue_families {
            if queue_config.num_queues[i] > 0 {
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

        let device_extension_names_raw = [ash::extensions::Swapchain::name().as_ptr()];

        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DeviceCreateInfo,
            p_next: ptr::null(),
            flags: Default::default(),
            queue_create_info_count: queue_create_info.len() as u32,
            p_queue_create_infos: queue_create_info.as_ptr(),
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            enabled_extension_count: device_extension_names_raw.len() as u32,
            pp_enabled_extension_names: device_extension_names_raw.as_ptr(),
            p_enabled_features: &features,
        };

        let vkd = unsafe {
            instance
                .pointers()
                .create_device(
                    physical_device_selection.physical_device,
                    &device_create_info,
                    None,
                )
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
            vk_khr_swapchain: extensions::Swapchain::new(instance.pointers(), &vkd)
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
            max_frames_in_flight,
            default_pool_block_size: default_alloc_block_size,
            default_pool: Mutex::new(Weak::new()),
            frame_fence: FrameFence::new(FrameNumber(1), max_frames_in_flight),
        })
    }
}