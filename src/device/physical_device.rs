//! Physical device selection
use std::ffi::{CStr, CString};

use ash::vk;
use instance::Instance;
use surface::Surface;

use super::QueueId;

pub(super) struct PhysicalDeviceSelection {
    pub(super) physical_device: vk::PhysicalDevice,
    pub(super) queue_family_properties: Vec<vk::QueueFamilyProperties>,
}

/// Selects a physical device compatible for presentation on the specified surface.
pub(super) fn select_physical_device(
    instance: &Instance,
    target_surface: Option<&Surface>,
) -> Result<PhysicalDeviceSelection, ()> {
    let physical_devices = instance
        .pointers()
        .enumerate_physical_devices()
        .expect("unable to enumerate physical devices");

    let mut selected_physical_device: Option<vk::PhysicalDevice> = None;
    let vk_khr_surface = instance.extension_pointers().vk_khr_surface;

    let mut compatible_physical_devices = physical_devices
        .iter()
        .cloned()
        .filter_map(|&physical_device| {
            // filter out incompatible physical devices
            let queue_family_properties = instance
                .pointers()
                .get_physical_device_queue_family_properties(physical_device);
            // check that the physical device has a queue that can present to the target surface, and that supports graphics
            let mut supports_surface = false;
            let mut supports_graphics = false;
            queue_family_properties
                .iter()
                .enumerate()
                .for_each(|(queue_family_index, info)| {
                    supports_graphics = info.queue_flags.subset(vk::QUEUE_GRAPHICS_BIT);
                    supports_surface = if let Some(surface) = surface {
                        instance
                            .extension_pointers()
                            .vk_khr_surface
                            .get_physical_device_surface_support_khr(
                                physical_device,
                                queue_family_index as u32,
                                surface,
                            )
                    } else {
                        true
                    };
                });

            if supports_graphics && supports_surface {
                Some((physical_device, queue_family_properties))
            } else {
                None
            }
        }).collect::<Vec<_>>();

    let (physical_device, queue_family_properties) =
        compatible_physical_devices.drain(..).next().ok_or(())?;

    // Print physical device name
    let dev_info = instance
        .pointers()
        .get_physical_device_properties(physical_device);
    let dev_name = unsafe {
        CStr::from_ptr(&dev_info.device_name[0])
            .to_owned()
            .into_string()
            .unwrap()
    };
    info!(
        "Using physical device: {} ({:?})",
        dev_name, dev_info.device_type
    );

    Ok(PhysicalDeviceSelection {
        physical_device,
        queue_family_properties,
    })
}

pub(super) struct DefaultQueueIds {
    pub(super) transfer: QueueId,
    pub(super) graphics: QueueId,
    pub(super) compute: QueueId,
    pub(super) present: QueueId,
}

pub(super) fn create_device_and_queues(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_properties: &[vk::QueueFamilyProperties],
    target_surface: Option<&Surface>,
) -> (vk::Device, DefaultQueueIds) {
    // first determine queue families for transfer, graphics, compute and present to target surface
    let transfer_queue_family =
        queue_family_properties
            .iter()
            .enumerate()
            .filter(|(queue_family_index, prop)| {
                // look for specialized transfer queues
                prop.queue_flags.subset(vk::QUEUE_TRANSFER_BIT)
            }).chain(queue_family_properties.iter().enumerate().filter(
                |(queue_family_index, prop)| {
                    // otherwise just use queues with GRAPHICS or COMPUTE capabilities
                    prop.queue_flags
                        .intersects(vk::QUEUE_GRAPHICS_BIT | vk::QUEUE_COMPUTE_BIT)
                },
            )).next()
            .expect("physical device does not have graphics or compute queues").0;

    let graphics_queue_family = queue_family_properties
        .iter()
        .enumerate()
        .filter(|(queue_family_index, prop)| prop.queue_flags.subset(vk::QUEUE_GRAPHICS_BIT))
        .next()
        .expect("unable to find a suitable graphics queue on selected device").0;

    let compute_queue_family = queue_family_properties
        .iter()
        .enumerate()
        .filter(|(queue_family_index, prop)| prop.queue_flags.subset(vk::QUEUE_COMPUTE_BIT))
        .next()
        .expect("unable to find a suitable compute queue on selected device").0;

    let present_queue_family = if let Some(surface) = surface {
        queue_family_properties
            .iter()
            .enumerate()
            .filter(|(queue_family_index, prop)| {
                instance
                    .extension_pointers()
                    .vk_khr_surface
                    .get_physical_device_surface_support_khr(
                        physical_device,
                        queue_family_index as u32,
                        surface,
                    )
            }).next().expect("unable to find a suitable queue for presentation on selected device").0
    } else {
        graphics_queue_family
    };

    info!("transfer queue family: {}", transfer_queue_family);
    info!("graphics queue family: {}", graphics_queue_family);
    info!("compute queue family: {}", compute_queue_family);
    info!("present queue family: {}", present_queue_family);

    let mut num_queues_per_family = vec![0; queue_family_properties.size()];
    num_queues_per_family[transfer_queue_family] += 1;
    num_queues_per_family[graphics_queue_family] += 1;
    num_queues_per_family[compute_queue_family] += 1;
    num_queues_per_family[present_queue_family] += 1;

    // 

    // create the device and the queues

    //

    // now, determine how many queues we are going to create: default is one for each category
}
