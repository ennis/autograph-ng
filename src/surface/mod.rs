use std::mem;
use std::ptr;
use std::sync::Arc;

use ash::extensions;
use ash::vk;
use config::Config;
use winit::Window;

use instance::{Instance, VkEntry1, VkInstance1};

mod native;

pub struct Surface {
    surface: vk::SurfaceKHR,
}

impl Surface {
    pub fn from_window(instance: &Arc<Instance>, window: &Window, cfg: &Config) -> Arc<Surface> {
        // create the native surface
        let surface =
            native::create_surface(instance.entry_pointers(), instance.pointers(), window)
                .expect("unable to create surface");

        Arc::new(Surface { surface })
    }
}
