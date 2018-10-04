mod swapchain;

#[derive(Clone)]
pub enum PresentationTarget {
    Window(Rc<Window>),
}

//--------------------------------------------------------------------------------------------------
// PRESENTATION

/// Resources associated to a presentation target.
pub struct Presentation {
    /// Presentation target.
    target: PresentationTarget,
    /// The surface: initialized when creating the context.
    surface: vk::SurfaceKHR,
    /// The swapchain: initialized when creating the context.
    swapchain: vk::SwapchainKHR,
    /// Images in the swapchain.
    images: Vec<Image>,
}

impl Presentation {
    /// Destroys the resources associated with the presentation object.
    /// Returns a reference to the presentation target passed on creation,
    /// for an eventual re-use.
    pub(crate) unsafe fn destroy(
        mut self,
        vkd: &VkDevice1,
        surface_ext: &extensions::Surface,
        swapchain_ext: &extensions::Swapchain,
    ) -> PresentationTarget {
        // destroy image views
        for img in self.images.drain(..) {
            img.image.unwrap().destroy(|img| {
                vkd.destroy_image(img, None);
            });
        }
        // destroy swapchain
        swapchain_ext.destroy_swapchain_khr(self.swapchain, None);
        // delete surface
        surface_ext.destroy_surface_khr(self.surface, None);
        self.target
    }

    /// Recreates the swap chain. To be called after receiving a `VK_ERROR_OUT_OF_DATE_KHR` result.
    pub(crate) unsafe fn recreate_swapchain(
        &mut self,
        vke: &VkEntry1,
        vki: &VkInstance1,
        vkd: &VkDevice1,
        surface_ext: &extensions::Surface,
        swapchain_ext: &extensions::Swapchain,
        physical_device: vk::PhysicalDevice,
    ) {
        // destroy image views
        for img in self.images.drain(..) {
            img.image.unwrap().destroy(|img| {
                vkd.destroy_image(img, None);
            });
        }
        // destroy swapchain
        swapchain_ext.destroy_swapchain_khr(self.swapchain, None);

        match self.target {
            PresentationTarget::Window(ref window) => {
                let hidpi_factor = window.get_hidpi_factor();
                let (window_width, window_height): (u32, u32) = window
                    .get_inner_size()
                    .unwrap()
                    .to_physical(hidpi_factor)
                    .into();
                // re-create swapchain
                self.swapchain = create_swapchain(
                    vke,
                    vki,
                    vkd,
                    surface_ext,
                    swapchain_ext,
                    physical_device,
                    window_width,
                    window_height,
                    self.surface,
                ).0;
            }
        }
    }
}

/// Helper function to create a swapchain.
pub(crate) fn create_swapchain(
    vke: &VkEntry1,
    vki: &VkInstance1,
    vkd: &VkDevice1,
    surface_loader: &extensions::Surface,
    swapchain_loader: &extensions::Swapchain,
    physical_device: vk::PhysicalDevice,
    window_width: u32,
    window_height: u32,
    surface: vk::SurfaceKHR,
) -> (vk::SwapchainKHR, vk::SwapchainCreateInfoKHR) {
    let surface_formats = surface_loader
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
    let surface_capabilities = surface_loader
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
        let swapchain = swapchain_loader
            .create_swapchain_khr(&swapchain_create_info, None)
            .unwrap();
        (swapchain, swapchain_create_info)
    }
}
