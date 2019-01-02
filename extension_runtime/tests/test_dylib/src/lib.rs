#![feature(proc_macro_hygiene)]
#![feature(allocator_api)]
use gfx2_extension_runtime::hot_reload_module;

use std::sync::atomic::AtomicPtr;
use std::alloc::Global;

#[hot_reload_module]
pub mod hot {

    #[no_mangle]
    pub extern "C" fn get_first(v: &Vec<i32>) -> &i32 {
        &v[0]
    }

    #[no_mangle]
    pub extern "C" fn simple(a: i32) -> Box<i32> {
        eprintln!("you called? {}", a);
        Box::new(a + 1)
    }
}
