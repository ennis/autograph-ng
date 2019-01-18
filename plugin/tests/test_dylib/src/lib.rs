#![feature(proc_macro_hygiene)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
use autograph_plugin::hot_reload_module;

#[hot_reload_module]
pub mod hot {
    #[no_mangle]
    pub extern "C" fn shorten_lifetime<'a, 'b, 'min>(a: &'a i32, b: &'b i32) -> &'min i32
    where
        'a: 'min,
        'b: 'min,
    {
        if *a > *b {
            a
        } else {
            b
        }
    }

    #[no_mangle]
    pub extern "C" fn push<'a, 'b>(v: &'a mut Vec<&'b i32>) {
        // even if this is static, this is still safe, because of the added bound
        v.push(&42);
    }

    #[no_mangle]
    pub extern "C" fn simple(a: i32) -> i32 {
        eprintln!("you called? {}", a);
        a + 1
    }

    #[no_mangle]
    pub static STRING: &str = "Hello!";
}
