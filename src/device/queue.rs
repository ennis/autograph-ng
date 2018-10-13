//! Queue selection
use ash;
use ash::vk;

use instance::Instance;
use surface::Surface;

pub(super) struct QueueConfiguration {
    pub(super) num_queues: Vec<u32>,
    pub(super) transfer: (u32, u32),
    pub(super) compute: (u32, u32),
    pub(super) graphics: (u32, u32),
    pub(super) present: (u32, u32),
}

/// Ideally, this should be controlled via hints by the application.
/// The algorithm here tries to find specialized queue families for compute, graphics, and transfer,
/// and create one different queue from each.
pub(super) fn create_queue_configuration(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_properties: &[vk::QueueFamilyProperties],
    target_surface: Option<&Surface>,
) -> QueueConfiguration {
    // first determine queue families for transfer, graphics, compute and present to target surface
    let transfer_family =
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
            .expect("physical device does not have graphics or compute queues")
            .0 as u32;

    let graphics_family = queue_family_properties
        .iter()
        .enumerate()
        .filter(|(queue_family_index, prop)| prop.queue_flags.subset(vk::QUEUE_GRAPHICS_BIT))
        .next()
        .expect("unable to find a suitable graphics queue on selected device")
        .0 as u32;

    let compute_family = queue_family_properties
        .iter()
        .enumerate()
        .filter(|(queue_family_index, prop)| prop.queue_flags.subset(vk::QUEUE_COMPUTE_BIT))
        .next()
        .expect("unable to find a suitable compute queue on selected device")
        .0 as u32;

    let present_family = if let Some(surface) = surface {
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
            }).next()
            .expect("unable to find a suitable queue for presentation on selected device")
            .0 as u32
    } else {
        graphics_family
    };

    info!("transfer queue family: {}", transfer_queue_family);
    info!("graphics queue family: {}", graphics_queue_family);
    info!("compute queue family: {}", compute_queue_family);
    info!("present queue family: {}", present_queue_family);

    let mut num_queues = vec![0; queue_family_properties.size()];

    // assign transfer queue
    let transfer = (transfer_family, num_queues[transfer_family as usize]);
    num_queues[transfer_family as usize] += 1;

    // assign graphics queue
    if num_queues[graphics_family as usize]
        < queue_family_properties[graphics_family as usize].queue_count
    {
        // create another one
        num_queues[graphics_family as usize] += 1;
    }
    let graphics = num_queues[graphics_family as usize] - 1;

    // assign compute queue
    if num_queues[compute_family as usize]
        < queue_family_properties[compute_family as usize].queue_count
    {
        num_queues[compute_family as usize] += 1;
    }
    let compute = num_queues[compute_family as usize] - 1;

    // assign present queue, preferably sharing with another queue of the same family.
    if num_queues[present_family as usize] == 0 {
        // no queues yet from this family, create one
        // this means that there is likely a specialized queue for presentation
        num_queues[present_family as usize] += 1;
    }
    let present = num_queues[present_family as usize] - 1;

    QueueConfiguration {
        num_queues,
        present,
        compute,
        graphics,
        transfer,
    }
}
