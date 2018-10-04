//! Abstraction over vulkan swapchains.

use ash::vk;

use image;

pub struct Swapchain
{
    device: Arc<Device>,
    surface: Arc<Surface>,
    swapchain: VkHandle<vk::SwapchainKHR>,
    images: Vec<Image>,
}

impl Swapchain
{
    pub fn new(device: &Arc<Device>, surface: &Arc<Surface>, width: u32, height: u32) -> Swapchain {
        let vkd = device.pointers();
        let vk_khr_surface = device.instance().extension_pointers().vk_khr_surface;
        let surface_formats = vk_khr_surface
            .get_physical_device_surface_formats_khr(physical_device, surface)
            .unwrap();
        let surface_format = surface_formats
            .iter()
            .map(|sfmt| match sfmt.format {
                vk::Format::Undefined => vk::SurfaceFormatKHR {
                    format: vk::Format::B8g8r8Unorm,
                    color_space: sfmt.color_space,
                },
                _ => sfmt.clone(),
            }).nth(0)
            .expect("Unable to find a suitable surface format");

        let surface_capabilities = vk_khr_surface
            .get_physical_device_surface_capabilities_khr(physical_device, surface)
            .unwrap();
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: window_width,
                height: window_height,
            },
            _ => surface_capabilities.current_extent,
        };
        let pre_transform = if surface_capabilities
            .supported_transforms
            .subset(vk::SURFACE_TRANSFORM_IDENTITY_BIT_KHR)
            {
                vk::SURFACE_TRANSFORM_IDENTITY_BIT_KHR
            } else {
            surface_capabilities.current_transform
        };
        let present_modes = surface_loader
            .get_physical_device_surface_present_modes_khr(physical_device, surface)
            .unwrap();
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::Mailbox)
            .unwrap_or(vk::PresentModeKHR::Fifo);
        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SwapchainCreateInfoKhr,
            p_next: ptr::null(),
            flags: Default::default(),
            surface,
            min_image_count: desired_image_count,
            image_color_space: surface_format.color_space,
            image_format: surface_format.format,
            image_extent: surface_resolution.clone(),
            image_usage: vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
            image_sharing_mode: vk::SharingMode::Exclusive,
            pre_transform,
            composite_alpha: vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
            present_mode,
            clipped: 1,
            old_swapchain: vk::SwapchainKHR::null(),
            image_array_layers: 1,
            p_queue_family_indices: ptr::null(),
            queue_family_index_count: 0,
        };
        unsafe {
            let swapchain = vk_khr_swapchain
                .create_swapchain_khr(&swapchain_create_info, None)
                .unwrap();
            swapchain
        }
    }
}