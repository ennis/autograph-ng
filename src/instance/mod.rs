use std::ffi::{CStr, CString};
use std::mem;
use std::ops::Deref;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use std::u32;

use ash;
use ash::extensions;
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0, V1_0};
use ash::vk;
use config::Config;

//--------------------------------------------------------------------------------------------------
pub type VkEntry1 = ash::Entry<V1_0>;
pub type VkInstance1 = ash::Instance<V1_0>;

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
pub struct Instance {
    entry: VkEntry1,
    pointers: VkInstance1,
    extension_pointers: ExtensionPointers,
}

pub struct ExtensionPointers {
    pub vk_ext_debug_report: ash::extensions::DebugReport,
    pub vk_khr_surface: ash::extensions::Surface,
}

/*impl Deref for Instance
{
    type Target = VkInstance1;

    fn deref(&self) -> &VkInstance1 {
        &self.pointers
    }
}*/

impl Instance {
    pub fn entry_pointers(&self) -> &VkEntry1 {
        &self.entry
    }

    pub fn pointers(&self) -> &VkInstance1 {
        &self.pointers
    }

    pub fn extension_pointers(&self) -> &ExtensionPointers {
        &self.extension_pointers
    }

    pub fn new(cfg: &Config) -> Arc<Instance> {
        // Load settings
        let vk_instance_extensions = cfg
            .get::<Vec<String>>("gfx.vulkan.instance_extensions")
            .unwrap();
        let vk_layers = cfg.get::<Vec<String>>("gfx.vulkan.layers").unwrap();
        let vk_default_alloc_block_size = cfg
            .get::<u64>("gfx.vulkan.default_alloc_block_size")
            .unwrap();

        unsafe {
            let vke = VkEntry1::new().unwrap();
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
            let vki: VkInstance1 = vke
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
                extensions::Surface::new(vke, vki).expect("unable to load surface extension");

            Arc::new(Instance {
                entry: vke,
                pointers: vki,
                extension_pointers: ExtensionPointers {
                    vk_khr_surface,
                    vk_ext_debug_report,
                },
            })
        }
    }
}
