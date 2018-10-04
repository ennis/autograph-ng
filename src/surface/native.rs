use std::mem;
use std::ptr;

use ash::extensions;
use ash::vk;
use winit::Window;

use instance::{Instance, VkEntry1, VkInstance1};

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
        p_view: window.get_nsview() as *const vk::types::c_void,
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
