//! Abstraction over vulkan swapchains.
use std::cell::Cell;
use std::cmp::max;
use std::ptr;
use std::sync::Arc;
use std::{u32, u64};

use ash::extensions;
use ash::vk;

use crate::device::{
    Device, FrameNumber, FrameSynchronizedObject, SharingMode, INVALID_FRAME_NUMBER,
};
use crate::handle::VkHandle;
use crate::image::generic::GenericImage;
use crate::image::traits::*;
use crate::image::Dimensions;
use crate::instance::Instance;
use crate::surface::Surface;
use crate::sync::FrameLock;

#[derive(Debug)]
pub enum SwapchainCreationError {
    UnsupportedFormat,
    UnsupportedImageCount,
}

//#[derive(Debug)]
pub struct Swapchain {
    device: Arc<Device>,
    surface: Arc<Surface>,
    swapchain: VkHandle<vk::SwapchainKHR>,
    images: Vec<vk::Image>,
    format: vk::Format,
    color_space: vk::ColorSpaceKHR,
    width: u32,
    height: u32,
    layers: u32,
    usage: vk::ImageUsageFlags,
    sharing: vk::SharingMode,
    transform: vk::SurfaceTransformFlagsKHR,
    alpha: vk::CompositeAlphaFlagsKHR,
    mode: vk::PresentModeKHR,
    clipped: bool,
    frame_lock: FrameLock,
}

impl Swapchain {
    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    pub fn internal_handle(&self) -> vk::SwapchainKHR {
        self.swapchain.get()
    }

    pub fn new(
        device: &Arc<Device>,
        surface: &Arc<Surface>,
        num_images: u32,
        format: vk::Format,
        color_space: vk::ColorSpaceKHR,
        dimensions: (u32, u32),
        layers: u32,
        usage: vk::ImageUsageFlags,
        transform: vk::SurfaceTransformFlagsKHR,
        alpha: vk::CompositeAlphaFlagsKHR,
        old_swapchain: Option<&Arc<Swapchain>>,
    ) -> Result<Arc<Swapchain>, SwapchainCreationError> {
        let vk_khr_surface = &device.instance().extension_pointers().vk_khr_surface;
        let physical_device = device.physical_device();

        let surface_formats = vk_khr_surface
            .get_physical_device_surface_formats_khr(physical_device, surface.internal_handle())
            .unwrap();

        let surface_format = surface_formats
            .iter()
            .map(|sfmt| match sfmt.format {
                vk::Format::Undefined => vk::SurfaceFormatKHR {
                    format,
                    color_space: sfmt.color_space,
                },
                _ => sfmt.clone(),
            })
            .find(|sfmt| sfmt.format == format)
            .ok_or(SwapchainCreationError::UnsupportedFormat)?;

        let surface_capabilities = surface.capabilities(physical_device);

        let num_images = max(num_images, surface_capabilities.min_image_count);
        if num_images > surface_capabilities.max_image_count {
            return Err(SwapchainCreationError::UnsupportedImageCount);
        }

        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: dimensions.0,
                height: dimensions.1,
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

        let present_mode = surface
            .present_modes(physical_device)
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::Mailbox)
            .unwrap_or(vk::PresentModeKHR::Fifo);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SwapchainCreateInfoKhr,
            p_next: ptr::null(),
            flags: Default::default(),
            surface: surface.internal_handle(),
            min_image_count: num_images,
            image_color_space: surface_format.color_space,
            image_format: surface_format.format,
            image_extent: surface_resolution.clone(),
            image_usage: usage,
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
        let swapchain = unsafe {
            device
                .extension_pointers()
                .vk_khr_swapchain
                .create_swapchain_khr(&swapchain_create_info, None)
                .expect("unable to create swapchain")
        };

        let images = device
            .extension_pointers()
            .vk_khr_swapchain
            .get_swapchain_images_khr(swapchain)
            .unwrap();
        /*let images = swapchain_images.iter().map(|img| {
            Arc::new(SwapchainImage::new())
        });*/

        Ok(Arc::new(Swapchain {
            device: device.clone(),
            surface: surface.clone(),
            swapchain: VkHandle::new(swapchain),
            images,
            format,
            color_space,
            width: 0,
            height: 0,
            layers: 0,
            usage,
            sharing: vk::SharingMode::Exclusive,
            transform: pre_transform,
            alpha,
            mode: present_mode,
            clipped: false,
            frame_lock: FrameLock::new(device),
        }))
    }

    pub(crate) fn acquire_next_image(&self, image_available: vk::Semaphore) -> u32 {
        // get next semaphore
        let next_image = unsafe {
            self.device
                .extension_pointers()
                .vk_khr_swapchain
                .acquire_next_image_khr(
                    self.swapchain.get(),
                    u64::MAX,
                    image_available,
                    vk::Fence::null(),
                )
                .unwrap()
        };
        next_image
    }
}

/// Wrapper around an image from a swapchain images.
pub struct SwapchainImageProxy {
    swapchain: Arc<Swapchain>,
    index: Cell<Option<u32>>,
    image_available: vk::Semaphore,
    dimensions: (u32, u32),
    array_layers: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    samples: u32,
    layout: vk::ImageLayout,
}

impl SwapchainImageProxy {
    /// Creates a new swapchain image from a raw handle.
    pub fn new(
        swapchain: &Arc<Swapchain>,
        image_available: vk::Semaphore,
        dimensions: (u32, u32),
        array_layers: u32,
        samples: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        layout: vk::ImageLayout,
    ) -> SwapchainImageProxy {
        SwapchainImageProxy {
            swapchain: swapchain.clone(),
            index: Cell::new(None),
            image_available,
            dimensions,
            format,
            usage,
            samples,
            array_layers,
            layout,
        }
    }

    pub fn acquire(&self) -> (u32, vk::Image) {
        if let Some(index) = self.index.get() {
            (index, self.swapchain.images[index as usize])
        } else {
            let index = self.swapchain.acquire_next_image(self.image_available);
            self.index.set(Some(index));
            (index, self.swapchain.images[index as usize])
        }
    }

    pub fn swapchain(&self) -> vk::SwapchainKHR {
        self.swapchain.internal_handle()
    }

    pub fn swapchain_index(&self) -> u32 {
        let (index, _) = self.acquire();
        index
    }
}

impl ImageDescription for Swapchain {
    fn dimensions(&self) -> Dimensions {
        if self.layers > 1 {
            Dimensions::Dim2dArray {
                width: self.width,
                height: self.height,
                array_layers: self.layers,
            }
        } else {
            Dimensions::Dim2d {
                width: self.width,
                height: self.height,
            }
        }
    }

    fn mipmaps_count(&self) -> u32 {
        1
    }

    fn samples(&self) -> u32 {
        1
    }

    fn format(&self) -> vk::Format {
        self.format
    }

    fn usage(&self) -> vk::ImageUsageFlags {
        self.usage
    }
}

unsafe impl FrameSynchronizedObject for Arc<Swapchain> {
    type Proxy = SwapchainImageProxy;

    unsafe fn lock(
        &self,
        frame_number: FrameNumber,
    ) -> (
        SwapchainImageProxy,
        Option<vk::Semaphore>,
        Option<vk::Semaphore>,
    ) {
        /// XXX we repurpose the frame lock, this is an ugly hack.
        let (_, image_available) = self.frame_lock.lock(frame_number);

        let image = SwapchainImageProxy::new(
            &self.clone(),
            image_available,
            (0, 0),
            0,
            0,
            self.format,
            self.usage,
            vk::ImageLayout::Undefined,
        );

        (image, Some(image_available), None)
    }
}
