
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
