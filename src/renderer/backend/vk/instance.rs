use std::ffi::{CStr, CString};
use std::mem;
use std::ops::Deref;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use std::u32;

use ash;
use ash::extensions;
use ash::vk;
use config::Config;

//--------------------------------------------------------------------------------------------------
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

//--------------------------------------------------------------------------------------------------
pub struct InstanceAndExtensions {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub vk_ext_debug_report: ash::extensions::DebugReport,
    pub vk_khr_surface: ash::extensions::Surface,
}

pub fn create_instance(cfg: &Config) -> InstanceAndExtensions {
    // Load settings
    let vk_instance_extensions = cfg
        .get::<Vec<String>>("gfx.vulkan.instance_extensions")
        .unwrap();
    let vk_layers = cfg.get::<Vec<String>>("gfx.vulkan.layers").unwrap();
    let vk_default_alloc_block_size = cfg.get::<u64>("gfx.default_alloc_block_size").unwrap();

    unsafe {
        let vke = ash::Entry::new().unwrap();
        let app_raw_name = CStr::from_bytes_with_nul(b"Autograph/GFX\0")
            .unwrap()
            .as_ptr();

        let mut layer_names = Vec::new();
        layer_names.push(CString::new("VK_LAYER_LUNARG_standard_validation").unwrap());
        layer_names.extend(
            vk_layers
                .iter()
                .map(|name| CString::new(name.clone()).unwrap()),
        );
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
        let vki = vke
            .create_instance(&create_info, None)
            .expect("Instance creation error");

        let debug_info = vk::DebugReportCallbackCreateInfoEXT {
            s_type: vk::StructureType::DebugReportCallbackCreateInfoExt,
            p_next: ptr::null(),
            flags: vk::DEBUG_REPORT_ERROR_BIT_EXT
                | vk::DEBUG_REPORT_WARNING_BIT_EXT
                | vk::DEBUG_REPORT_PERFORMANCE_WARNING_BIT_EXT
                | vk::DEBUG_REPORT_DEBUG_BIT_EXT
                | vk::DEBUG_REPORT_INFORMATION_BIT_EXT,
            pfn_callback: vulkan_debug_callback,
            p_user_data: ptr::null_mut(),
        };

        //---------------------------------
        // set debug report callback
        let vk_ext_debug_report = extensions::DebugReport::new(&vke, &vki)
            .expect("unable to load debug report extension");
        let debug_callback = vk_ext_debug_report
            .create_debug_report_callback_ext(&debug_info, None)
            .unwrap();

        let vk_khr_surface =
            extensions::Surface::new(&vke, &vki).expect("unable to load surface extension");

        InstanceAndExtensions {
            entry: vke,
            instance: vki,
            vk_khr_surface,
            vk_ext_debug_report,
        }
    }
}

