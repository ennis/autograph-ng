//! Context creation
//! A `Context` wraps a vulkan instance, device, and swapchain.
use std::rc::Rc;
use std::ffi::{CString, CStr};
use std::ptr;
use std::os::raw::{c_char, c_void};
use std::mem;
use std::u32;
use std::cell::Cell;

use config::Config;
use ash;
use ash::vk;
use ash::extensions;
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0, V1_0};
use winit::Window;
use slotmap::{SlotMap, Key};

use upload_buffer::UploadBuffer;
use buffer::{BufferSlice, BufferDesc, BufferStorage};
use texture::{TextureDesc, TextureObject};
use frame::{TaskId, ResourceId};

pub type VkEntry1 = ash::Entry<V1_0>;
pub type VkInstance1 = ash::Instance<V1_0>;
pub type VkDevice1 = ash::Device<V1_0>;

#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
fn extension_names() -> Vec<*const c_char> {
    vec![
        extensions::Surface::name().as_ptr(),
        extensions::XlibSurface::name().as_ptr(),
        extensions::DebugReport::name().as_ptr(),
    ]
}

#[cfg(target_os = "macos")]
fn extension_names() -> Vec<*const c_char> {
    vec![
        extensions::Surface::name().as_ptr(),
        extensions::MacOSSurface::name().as_ptr(),
        extensions::DebugReport::name().as_ptr(),
    ]
}

#[cfg(all(windows))]
fn extension_names() -> Vec<*const c_char> {
    vec![
        extensions::Surface::name().as_ptr(),
        extensions::Win32Surface::name().as_ptr(),
        extensions::DebugReport::name().as_ptr(),
    ]
}

/// Debug callback for the vulkan debug report extension.
unsafe extern "system" fn vulkan_debug_callback(
    _: vk::DebugReportFlagsEXT,
    _: vk::DebugReportObjectTypeEXT,
    _: vk::uint64_t,
    _: vk::size_t,
    _: vk::int32_t,
    _: *const vk::c_char,
    p_message: *const vk::c_char,
    _: *mut vk::c_void,
) -> u32 {
    debug!("{:?}", CStr::from_ptr(p_message));
    vk::VK_FALSE
}

/// Return value of `create_device_and_queues`
struct DeviceAndQueues
{
    physical_device: vk::PhysicalDevice,
    vkd: VkDevice1,
    graphics_queue_family_index: u32,
    present_queue_family_index: u32,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
}

/// Helper function to create a vulkan device and queues that are compatible with
/// the specified surface (if any).
unsafe fn create_device_and_queues(
    vke: &VkEntry1,
    vki: &VkInstance1,
    surface_loader: &extensions::Surface,
    surface: Option<vk::SurfaceKHR>,
    config: &Config) -> DeviceAndQueues
{
    let physical_devices = vki
        .enumerate_physical_devices()
        .expect("Physical device error");

    // The selected physical device.
    // TODO prefer discrete graphics?
    let mut selected_physical_device: Option<vk::PhysicalDevice> = None;
    // The selected queue family index. Must support graphics.
    let mut selected_queue_family_index: Option<u32> = None;

    'outer: for physical_device in physical_devices.iter() {
        // Print physical device name
        let dev_info = vki.get_physical_device_properties(*physical_device);
        let dev_name  = unsafe { CStr::from_ptr(&dev_info.device_name[0]).to_owned().into_string().unwrap() };
        info!("Physical device: {}", dev_name);

        let queue_family_props = vki.get_physical_device_queue_family_properties(*physical_device);
        for (queue_family_index, ref queue_family_info) in queue_family_props.iter().enumerate() {
            info!("Queue family #{}: {:?}", queue_family_index, queue_family_info);
            // does the queue supports graphics?
            let supports_graphics = queue_family_info.queue_flags.subset(vk::QUEUE_GRAPHICS_BIT);

            // is the queue compatible with the surface we just created?
            let supports_surface =
                if let Some(surface) = surface {
                    surface_loader.get_physical_device_surface_support_khr(
                        *physical_device,
                        queue_family_index as u32,
                        surface,
                    )
                } else {
                    true
                };
            if supports_graphics && supports_surface {
                // OK, choose this queue and physical device.
                selected_physical_device = Some(*physical_device);
                selected_queue_family_index = Some(queue_family_index as u32);
                break 'outer;
            }
        }
    }

    if let Some(selected_physical_device) = selected_physical_device {
        let selected_queue_family_index = selected_queue_family_index.unwrap();
        // create the queue and the device
        let priorities = [1.0];
        let queue_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DeviceQueueCreateInfo,
            p_next: ptr::null(),
            flags: Default::default(),
            queue_family_index: selected_queue_family_index,
            p_queue_priorities: priorities.as_ptr(),
            queue_count: priorities.len() as u32,
        };

        let device_extension_names_raw = [extensions::Swapchain::name().as_ptr()];

        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DeviceCreateInfo,
            p_next: ptr::null(),
            flags: Default::default(),
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            enabled_extension_count: device_extension_names_raw.len() as u32,
            pp_enabled_extension_names: device_extension_names_raw.as_ptr(),
            p_enabled_features: &features,
        };

        let vkd = vki
            .create_device(selected_physical_device, &device_create_info, None)
            .expect("Unable to create device");
        let present_queue = vkd.get_device_queue(selected_queue_family_index as u32, 0);
        // let's assume that the queue is also a graphics queue...
        DeviceAndQueues {
            graphics_queue: present_queue,
            present_queue: present_queue,
            graphics_queue_family_index: selected_queue_family_index,
            present_queue_family_index: selected_queue_family_index,
            vkd,
            physical_device: selected_physical_device
        }

    } else {
        panic!("Unable to find a suitable physical device and queue family");
    }
}


/// Helper function to create a command pool for a given queue family.
unsafe fn create_command_pool_for_queue(vkd: &VkDevice1, queue_family_index: u32) -> vk::CommandPool
{
    let command_pool_create_info = vk::CommandPoolCreateInfo {
        s_type: vk::StructureType::CommandPoolCreateInfo,
        p_next: ptr::null(),
        flags: vk::COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
        queue_family_index,
    };

    vkd.create_command_pool(&command_pool_create_info, None).unwrap()
}

pub type FrameNumber = u64;


#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
pub(crate) unsafe fn create_surface(
    entry: &VkEntry1,
    instance: &VkInstance1,
    window: &Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    use winit::os::unix::WindowExt;
    let x11_display = window.get_xlib_display().unwrap();
    let x11_window = window.get_xlib_window().unwrap();
    let x11_create_info = vk::XlibSurfaceCreateInfoKHR {
        s_type: vk::StructureType::XlibSurfaceCreateInfoKhr,
        p_next: ptr::null(),
        flags: Default::default(),
        window: x11_window as vk::Window,
        dpy: x11_display as *mut vk::Display,
    };
    let xlib_surface_loader =
        extensions::XlibSurface::new(entry, instance).expect("Unable to load xlib surface");
    xlib_surface_loader.create_xlib_surface_khr(&x11_create_info, None)
}

#[cfg(target_os = "macos")]
pub(crate) unsafe fn create_surface(
    entry: &VkEntry1,
    instance: &VkInstance1,
    window: &Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    use winit::os::macos::WindowExt;
    let wnd: cocoa_id = mem::transmute(window.get_nswindow());
    let layer = CoreAnimationLayer::new();

    layer.set_edge_antialiasing_mask(0);
    layer.set_presents_with_transaction(false);
    layer.remove_all_animations();

    let view = wnd.contentView();

    layer.set_contents_scale(view.backingScaleFactor());
    view.setLayer(mem::transmute(layer.as_ref()));
    view.setWantsLayer(YES);

    let create_info = vk::MacOSSurfaceCreateInfoMVK {
        s_type: vk::StructureType::MacOSSurfaceCreateInfoMvk,
        p_next: ptr::null(),
        flags: Default::default(),
        p_view: window.get_nsview() as *const vk::types::c_void
    };

    let macos_surface_loader =
        extensions::MacOSSurface::new(entry, instance).expect("Unable to load macOS surface");
    macos_surface_loader.create_macos_surface_mvk(&create_info, None)
}

#[cfg(target_os = "windows")]
pub(crate) unsafe fn create_surface(
    entry: &VkEntry1,
    instance: &VkInstance1,
    window: &Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    use winapi::shared::minwindef::HINSTANCE;
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::{GetWindowLongW, GWL_HINSTANCE};
    use winit::os::windows::WindowExt;

    let hwnd = window.get_hwnd() as HWND;
    // dafuq?
    //let hinstance = GetWindow(hwnd, 0) as *const vk::c_void;
    let hinstance = GetWindowLongW(hwnd, GWL_HINSTANCE) as *const _;
    let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
        s_type: vk::StructureType::Win32SurfaceCreateInfoKhr,
        p_next: ptr::null(),
        flags: Default::default(),
        hinstance,
        hwnd: hwnd as *const _,
    };
    let win32_surface_loader =
        extensions::Win32Surface::new(entry, instance).expect("Unable to load win32 surface");
    win32_surface_loader.create_win32_surface_khr(&win32_create_info, None)
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
    surface: vk::SurfaceKHR) -> vk::SwapchainKHR
{
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
        })
        .nth(0)
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
        swapchain
    }
}

pub(crate) unsafe fn create_semaphore(vkd: &VkDevice1) -> vk::Semaphore
{
    let info = vk::SemaphoreCreateInfo {
        s_type: vk::StructureType::SemaphoreCreateInfo,
        flags: vk::SemaphoreCreateFlags::default(),
        p_next: ptr::null()
    };
    vkd.create_semaphore(&info, None).expect("failed to create semaphore")
}

/// Resources associated to a presentation target.
///
pub(crate) struct PresentationInternal
{
    /// Presentation target.
    pub(crate) target: PresentationTarget,
    /// The surface: initialized when creating the context.
    pub(crate) surface: vk::SurfaceKHR,
    /// The swapchain: initialized when creating the context.
    pub(crate) swapchain: vk::SwapchainKHR,
    /// Images in the swapchain.
    pub(crate) images: Vec<vk::Image>,
}

impl PresentationInternal
{
    /// Destroys the resources associated with the presentation object.
    /// Returns a reference to the presentation target passed on creation,
    /// for an eventual re-use.
    pub(crate) unsafe fn destroy(mut self,
                                 vkd: &VkDevice1,
                                 surface_ext: &extensions::Surface,
                                 swapchain_ext: &extensions::Swapchain) -> PresentationTarget
    {
        // destroy image views
        for img in self.images.drain(..) {
            vkd.destroy_image(img, None);
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
        physical_device: vk::PhysicalDevice)
    {
        // destroy image views
        for img in self.images.drain(..) {
            vkd.destroy_image(img, None);
        }
        // destroy swapchain
        swapchain_ext.destroy_swapchain_khr(self.swapchain, None);

        match self.target {
            PresentationTarget::Window(ref window) => {
                let hidpi_factor = window.get_hidpi_factor();
                let (window_width, window_height): (u32, u32) = window.get_inner_size().unwrap().to_physical(hidpi_factor).into();
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
                    self.surface);
            }
        }
    }

    /*/// Creates the resources associated with the presentation target.
    pub(crate) unsafe fn create(
        vkd: &VkDevice1,
        surface_ext: &extensions::Surface,
        swapchain_ext: &extensions::Swapchain,
        presentation_target: PresentationTarget,
        previous: Option<PresentationInternal>) -> PresentationInternal
    {
        // TODO if initialized, delete image views and swapchain
        // TODO create swapchain
        // TODO create image views
    }*/

    /*pub(crate) fn new(surface: vk::SurfaceKHR,
                      swapchain_loader: &extensions::Swapchain,
                      swapchain: vk::SwapchainKHR) -> PresentationTargetInternal
    {
        PresentationTargetInternal {
            surface,
            swapchain,
            images
        }
    }*/
}

type PresentationId = Key;

/// A `presentation` object bundles a vulkan surface, swapchain,
/// and a reference to the winit window.
/// Note: this is just a handle type. Dropping it won't free it's contents.
#[derive(Clone)]
pub struct Presentation
{
    /*/// The window for which we created a surface.
    /// Note: maybe it's possible to create surfaces without a window in vulkan?
    pub(crate) window: Option<Rc<Window>>,*/
    /// ID in the map of presentations owned by the context.
    pub(crate) id: PresentationId,
}

#[derive(Clone)]
pub enum PresentationTarget
{
    Window(Rc<Window>)
}


/// Main graphics context.
/// Handles allocation of persistent resources.
pub struct Context {
    ///// The main upload buffer, where transient resources such as dynamic uniform buffers are allocated.
    //pub(crate) upload_buffer: UploadBuffer,
    pub(crate) vke: VkEntry1,
    pub(crate) vki: VkInstance1,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) vkd: VkDevice1,
    pub(crate) graphics_queue_family_index: u32,
    pub(crate) present_queue_family_index: u32,
    pub(crate) graphics_queue: vk::Queue,
    pub(crate) present_queue: vk::Queue,
    pub(crate) graphics_queue_command_pool: vk::CommandPool,
    pub(crate) present_queue_command_pool: vk::CommandPool,
    pub(crate) surface_loader: extensions::Surface,
    pub(crate) swapchain_loader: extensions::Swapchain,
    pub(crate) presentations: SlotMap<PresentationInternal>,
    pub(crate) max_in_flight_frames: u8,
    pub(crate) image_available: vk::Semaphore,
    pub(crate) render_finished: vk::Semaphore,
}

impl Context {

    /// Creates a new context and associated `Presentation` objects.
    pub fn new(presentation_targets: &[&PresentationTarget], cfg: &Config) -> (Context, Vec<Presentation>)
    {
        // Load settings
        let initial_upload_buffer_size = cfg.get::<usize>("gfx.default_upload_buffer_size").unwrap();
        let max_in_flight_frames = cfg.get::<usize>("gfx.max_in_flight_frames").unwrap();
        let vk_instance_extensions = cfg.get::<Vec<String>>("gfx.vulkan.instance_extensions").unwrap();
        let vk_layers = cfg.get::<Vec<String>>("gfx.vulkan.layers").unwrap();

        unsafe {
            let vke = VkEntry1::new().unwrap();
            let app_raw_name = CStr::from_bytes_with_nul(b"Autograph/GFX\0").unwrap().as_ptr();

            let mut layer_names = Vec::new();
            layer_names.push(CString::new("VK_LAYER_LUNARG_standard_validation").unwrap());
            layer_names.extend(vk_layers.iter().map(|name| CString::new(name.clone()).unwrap()));
            let layers_names_raw: Vec<*const c_char> = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            let extension_names_raw = extension_names();
            let appinfo = vk::ApplicationInfo {
                p_application_name: app_raw_name,
                s_type: vk::StructureType::ApplicationInfo,
                p_next: ptr::null(),
                application_version: 0,
                p_engine_name: app_raw_name,
                engine_version: 0,
                api_version: vk_make_version!(1, 0, 36),
            };
            let create_info = vk::InstanceCreateInfo {
                s_type: vk::StructureType::InstanceCreateInfo,
                p_next: ptr::null(),
                flags: Default::default(),
                p_application_info: &appinfo,
                pp_enabled_layer_names: layers_names_raw.as_ptr(),
                enabled_layer_count: layers_names_raw.len() as u32,
                pp_enabled_extension_names: extension_names_raw.as_ptr(),
                enabled_extension_count: extension_names_raw.len() as u32,
            };
            let vki: VkInstance1 = vke
                .create_instance(&create_info, None)
                .expect("Instance creation error");

            let debug_info = vk::DebugReportCallbackCreateInfoEXT {
                s_type: vk::StructureType::DebugReportCallbackCreateInfoExt,
                p_next: ptr::null(),
                flags: vk::DEBUG_REPORT_ERROR_BIT_EXT | vk::DEBUG_REPORT_WARNING_BIT_EXT
                    | vk::DEBUG_REPORT_PERFORMANCE_WARNING_BIT_EXT | vk::DEBUG_REPORT_DEBUG_BIT_EXT | vk::DEBUG_REPORT_INFORMATION_BIT_EXT,
                pfn_callback: vulkan_debug_callback,
                p_user_data: ptr::null_mut(),
            };

            //---------------------------------
            // set debug report callback
            let debug_report_loader = extensions::DebugReport::new(&vke, &vki).expect("Unable to load debug report");
            let debug_call_back = debug_report_loader
                .create_debug_report_callback_ext(&debug_info, None)
                .unwrap();

            //---------------------------------
            // we have an instance, now create a device that best fits the presentation target
            assert!(presentation_targets.len() <= 1, "Cannot yet specify more than one presentation target");

            // create surfaces for each presentation target
            let mut surfaces = Vec::new();
            for t in presentation_targets {
                let surf = match t {
                    PresentationTarget::Window(ref window) => {
                        create_surface(&vke, &vki, window).expect("Unable to create a surface")
                    },
                    _ => {
                        panic!("Cannot create a surface without a window");
                    }
                };
                surfaces.push(surf)
            }

            // create device and queues
            let surface_loader = extensions::Surface::new(&vke, &vki).expect("Unable to load surface extension");
            let device_and_queues = create_device_and_queues(&vke, &vki, &surface_loader, surfaces.first().cloned(), cfg);

            //---------------------------------
            // create swapchains for each initial presentation target
            let swapchain_loader =
                extensions::Swapchain::new(&vki, &device_and_queues.vkd).expect("Unable to load swapchain extension");

            let mut presentations = SlotMap::new();
            let mut presentation_handles = Vec::new();

            for (i,t) in presentation_targets.iter().enumerate() {
                let surface = surfaces[i];
                match t {
                    PresentationTarget::Window(ref window) => {
                        let hidpi_factor = window.get_hidpi_factor();
                        let (window_width, window_height): (u32, u32) = window.get_inner_size().unwrap().to_physical(hidpi_factor).into();
                        // FIXME: should put swapchain parameters in PresentationTarget
                        let swapchain = create_swapchain(
                            &vke, &vki, &device_and_queues.vkd,
                            &surface_loader, &swapchain_loader,
                            device_and_queues.physical_device,
                            window_width, window_height,
                            surface);
                        let swapchain_images = swapchain_loader.get_swapchain_images_khr(swapchain).unwrap();

                        presentation_handles.push(
                            Presentation {
                                //window: Some(window.clone()),
                                id: presentations.insert(PresentationInternal {
                                    target: (*t).clone(),
                                    surface,
                                    swapchain,
                                    images: swapchain_images
                                })
                            });
                    },
                    _ => panic!("Cannot create a swapchain without a window")
                }
            }

            //---------------------------------
            // create command pools
            let present_pool = create_command_pool_for_queue(&device_and_queues.vkd, device_and_queues.present_queue_family_index);
            let graphics_pool = create_command_pool_for_queue(&device_and_queues.vkd, device_and_queues.graphics_queue_family_index);

            //---------------------------------
            // create semaphores to sync between the draw and present queues
            let image_available = create_semaphore(&device_and_queues.vkd);
            let render_finished = create_semaphore(&device_and_queues.vkd);

            (Context {
                vke,
                vki,
                presentations,
                physical_device: device_and_queues.physical_device,
                vkd: device_and_queues.vkd,
                graphics_queue_family_index: device_and_queues.graphics_queue_family_index,
                present_queue_family_index: device_and_queues.present_queue_family_index,
                graphics_queue: device_and_queues.graphics_queue,
                present_queue: device_and_queues.present_queue,
                graphics_queue_command_pool: graphics_pool,
                present_queue_command_pool: present_pool,
                surface_loader,
                swapchain_loader,
                max_in_flight_frames: max_in_flight_frames as u8,
                image_available,
                render_finished
            }, presentation_handles)
        }
    }

    /// Reinitializes a presentation object.
    pub fn reset_presentation(&mut self, presentation: Presentation)
    {
        // TODO: wait for all commands complete before deleting the resources associated with the presentation.
        let presentation = self.presentations.get_mut(presentation.id).expect("invalid presentation handle");
        unsafe {
            presentation.recreate_swapchain(
                &self.vke,
                &self.vki,
                &self.vkd,
                &self.surface_loader,
                &self.swapchain_loader,
                self.physical_device);
        }
    }

    /*/// Acquires a presentation image.
    pub fn acquire_presentation_image(&mut self, presentation: Presentation) ->
    {

    }*/


    /*/// Initializes OR re-initializes a presentation target.
    fn initialize_presentation_target(&self, target: &PresentationTarget)
    {
        if let Some(id) = target.index {
            let target =

        } else {
            unimplemented!()
        }
    }*/

    /*/// Creates a frame.
    pub fn create_frame<'a>(&'a self, target: &PresentationTarget) -> Frame<'a> {
        unimplemented!()
    }*/

    /*/// Returns information about a texture resource from an ID.
    pub fn get_resource_info(&self, resource: &ResourceRef) -> ResourceInfo {
        unimplemented!()
    }

    /// Creates a persistent texture, with optional initial data.
    pub fn create_texture(&mut self, desc: &TextureDesc) -> Texture {
        unimplemented!()
    }

    /// Creates a persistent buffer.
    pub fn create_buffer(&mut self, desc: &BufferDesc) -> Buffer {
        unimplemented!()
    }*/

}
