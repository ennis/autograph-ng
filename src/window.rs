//! Boilerplate code for creating a window and an OpenGL context with winit/glutin.

use std::ffi::{CString, CStr};
use std::ptr;
use std::os::raw::{c_char, c_void};
use std::mem;

use winit;
use config;
use ash;
use ash::extensions;
use ash::vk;
use pretty_env_logger;

pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0, V1_0};

use context;

// re-export window event handling stuff.
pub use winit::EventsLoop;
pub use winit::WindowBuilder;
pub use winit::Window;
pub use winit::{Event,
                 WindowEvent,
                 MouseButton,
                 MouseScrollDelta,
                 KeyboardInput,
                 VirtualKeyCode,
                 ElementState,
                 ModifiersState,
                 DeviceId,
                 AxisId,
                 Touch,
                 ButtonId,
                 TouchPhase,
                 dpi::{LogicalPosition,
                       LogicalSize,
                       PhysicalPosition,
                       PhysicalSize}};

pub struct App
{
    pub cfg: config::Config,
    pub events_loop: winit::EventsLoop,
    pub window: winit::Window,
    pub context: context::Context,
}

impl App
{
    pub fn new() -> App {
        pretty_env_logger::init();
        let mut cfg = config::Config::default();
        cfg.merge(config::File::with_name("Settings")).unwrap();
        load_environment_config(&mut cfg);
        let mut events_loop = create_events_loop();
        let (mut window, mut context) = create_main_window_and_context(&events_loop, &cfg);

        App {
            cfg,
            events_loop,
            window,
            context
        }
    }
}

pub fn load_environment_config(cfg: &mut config::Config)
{
    cfg.merge(config::Environment::with_prefix("GFX")).unwrap();
}

pub fn create_events_loop() -> EventsLoop
{
    winit::EventsLoop::new()
}

#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
fn extension_names() -> Vec<*const c_char> {
    vec![
        ash::extensions::Surface::name().as_ptr(),
        ash::extensions::XlibSurface::name().as_ptr(),
        ash::extensions::DebugReport::name().as_ptr(),
    ]
}

#[cfg(target_os = "macos")]
fn extension_names() -> Vec<*const c_char> {
    vec![
        ash::extensions::Surface::name().as_ptr(),
        ash::extensions::MacOSSurface::name().as_ptr(),
        ash::extensions::DebugReport::name().as_ptr(),
    ]
}

#[cfg(all(windows))]
fn extension_names() -> Vec<*const c_char> {
    vec![
        ash::extensions::Surface::name().as_ptr(),
        ash::extensions::Win32Surface::name().as_ptr(),
        ash::extensions::DebugReport::name().as_ptr(),
    ]
}

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
    info!("{:?}", CStr::from_ptr(p_message));
    vk::VK_FALSE
}

#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
unsafe fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
    entry: &E,
    instance: &I,
    window: &winit::Window,
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
unsafe fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
    entry: &E,
    instance: &I,
    window: &winit::Window,
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
unsafe fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
    entry: &E,
    instance: &I,
    window: &winit::Window,
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

pub fn create_vk_device(instance: &ash::Instance<V1_0>, config: &config::Config)
{
    let pdevices = instance
        .enumerate_physical_devices()
        .expect("Physical device error");
    //let surface_loader =
    //    Surface::new(&entry, &instance).expect("Unable to load the Surface extension");

    for pdev in pdevices.iter() {
        // Print physical device name
        let pdev_info = instance.get_physical_device_properties(*pdev);
        let pdev_name  = unsafe { CStr::from_ptr(&pdev_info.device_name[0]).to_owned().into_string().unwrap() };
        info!("Physical device: {}", pdev_name);
        debug!("{:?}", pdev_info);

        let queue_family_props = instance.get_physical_device_queue_family_properties(*pdev);
        for (index, ref queue_family_info) in queue_family_props.iter().enumerate() {
            debug!("Queue family #{}: {:?}", index, queue_family_info);
        }
    }
}

pub fn create_main_window_and_context(events_loop: &EventsLoop, cfg: &config::Config) -> (Window, context::Context)
{
    // Load settings
    let window_width = cfg.get::<u32>("gfx.window.width").unwrap();
    let window_height = cfg.get::<u32>("gfx.window.height").unwrap();
    let fullscreen = cfg.get::<u32>("gfx.window.fullscreen").unwrap();
    let vsync = cfg.get::<bool>("gfx.window.vsync").unwrap();
    let window_title = cfg.get::<String>("gfx.window.title").unwrap();
    //let gl_version_major = config.get::<u8>("gfx.gl.version_major").unwrap();
    //let gl_version_minor = config.get::<u8>("gfx.gl.version_minor").unwrap();
    //let gl_debug = config.get::<bool>("gfx.gl.debug").unwrap();
    let initial_upload_buffer_size = cfg.get::<usize>("gfx.default_upload_buffer_size").unwrap();
    let max_in_flight_frames = cfg.get::<usize>("gfx.max_in_flight_frames").unwrap();
    let vk_instance_extensions = cfg.get::<Vec<String>>("gfx.vulkan.instance_extensions").unwrap();
    let vk_layers = cfg.get::<Vec<String>>("gfx.vulkan.layers").unwrap();

    // create a window
    let window_builder = winit::WindowBuilder::new()
        .with_title(window_title.clone())
        .with_dimensions((window_width, window_height).into());
    let window = window_builder.build(events_loop).unwrap();

    //
    unsafe {
        let entry = ash::Entry::<V1_0>::new().unwrap();
        let app_name = CString::new(window_title).unwrap();
        let raw_name = app_name.as_ptr();

        let mut layer_names = Vec::new();
        layer_names.push(CString::new("VK_LAYER_LUNARG_standard_validation").unwrap());
        layer_names.extend(vk_layers.iter().map(|name| CString::new(name.clone()).unwrap()));
        let layers_names_raw: Vec<*const c_char> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let extension_names_raw = extension_names();
        let appinfo = vk::ApplicationInfo {
            p_application_name: raw_name,
            s_type: vk::StructureType::ApplicationInfo,
            p_next: ptr::null(),
            application_version: 0,
            p_engine_name: raw_name,
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
        let instance: ash::Instance<V1_0> = entry
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

        let debug_report_loader =
            ash::extensions::DebugReport::new(&entry, &instance).expect("Unable to load debug report");
        let debug_call_back = debug_report_loader
            .create_debug_report_callback_ext(&debug_info, None)
            .unwrap();

        let surface = create_surface(&entry, &instance, &window);
        create_vk_device(&instance, cfg);
    }

    (window, context::Context::new())
}