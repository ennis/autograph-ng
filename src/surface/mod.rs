use std::mem;
use std::ptr;
use std::sync::Arc;

use ash::extensions;
use ash::vk;
use config::Config;
use winit::Window;

use crate::instance::{Instance, VkEntry1, VkInstance1};

mod native;

pub struct Surface {
    instance: Arc<Instance>,
    surface: vk::SurfaceKHR,
}

impl Surface {
    pub fn from_window(instance: &Arc<Instance>, window: &Window, cfg: &Config) -> Arc<Surface> {
        // create the native surface
        let surface = unsafe {
            native::create_surface(instance.entry_pointers(), instance.pointers(), window)
                .expect("unable to create surface")
        };

        Arc::new(Surface {
            instance: instance.clone(),
            surface,
        })
    }

    pub fn internal_handle(&self) -> vk::SurfaceKHR {
        self.surface
    }

    pub fn capabilities(&self, physical_device: vk::PhysicalDevice) -> vk::SurfaceCapabilitiesKHR {
        self.instance
            .extension_pointers()
            .vk_khr_surface
            .get_physical_device_surface_capabilities_khr(physical_device, self.surface)
            .unwrap()
    }

    pub fn supported_formats(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Vec<vk::SurfaceFormatKHR> {
        self.instance
            .extension_pointers()
            .vk_khr_surface
            .get_physical_device_surface_formats_khr(physical_device, self.surface)
            .unwrap()
    }

    pub fn present_modes(&self, physical_device: vk::PhysicalDevice) -> Vec<vk::PresentModeKHR> {
        self.instance
            .extension_pointers()
            .vk_khr_surface
            .get_physical_device_surface_present_modes_khr(physical_device, self.surface)
            .unwrap()
    }
}
