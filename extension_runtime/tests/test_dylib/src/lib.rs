#![feature(proc_macro_hygiene)]
use gfx2_extension_runtime::hot_reload_module;

//#[hot_reload_module]
pub mod hot {

    #[doc(hidden)]
    pub mod __load {
        pub struct DllShims<'__lib> {
            fnptr_shorten_lifetime: libloading::Symbol<'__lib, *const ::std::ffi::c_void>,
            fnptr_get_first: ::libloading::Symbol<'__lib, *const ::std::ffi::c_void>,
            fnptr_simple: ::libloading::Symbol<'__lib, *const ::std::ffi::c_void>,
        }
        impl<'__lib> DllShims<'__lib> {
            pub fn shorten_lifetime<'a, 'b, 'min>(&self, arg0: &'a i32, arg1: &'b i32) -> &'min i32
                where
                    'a: 'min,
                    'b: 'min,
            {
                (unsafe {
                    ::std::mem::transmute::<_, fn(a: &'a i32, b: &'b i32) -> &'min i32>(
                        *self.fnptr_shorten_lifetime,
                    )
                })(arg0, arg1)
            }
            pub fn get_first(&self, arg0: &Vec<i32>) -> &i32 {
                (unsafe {
                    ::std::mem::transmute::<_, fn(v: &Vec<i32>) -> &i32>(*self.fnptr_get_first)
                })(arg0)
            }
            pub fn simple(&self, arg0: i32) -> i32 {
                (unsafe { ::std::mem::transmute::<_, fn(a: i32) -> i32>(*self.fnptr_simple) })(arg0)
            }
            pub fn load(lib: &'__lib ::libloading::Library) -> ::libloading::Result<Self> {
                Ok(Self {
                    fnptr_shorten_lifetime: unsafe {
                        lib.get(stringify!(shorten_lifetime).as_bytes())?
                    },
                    fnptr_get_first: unsafe { lib.get(stringify!(get_first).as_bytes())? },
                    fnptr_simple: unsafe { lib.get(stringify!(simple).as_bytes())? },
                })
            }
        }
    }

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
    pub extern "C" fn get_first(v: &Vec<i32>) -> &i32 {
        &v[0]
    }

    #[no_mangle]
    pub extern "C" fn simple(a: i32) -> i32 {
        eprintln!("you called? {}", a);
        a + 1
    }
}
