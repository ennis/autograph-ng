//! Physical device selection
use std::ffi::{CStr, CString};
use std::ptr;

use sid_vec::IdVec;

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
