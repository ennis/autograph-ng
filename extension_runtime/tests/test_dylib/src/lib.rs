#![feature(proc_macro_hygiene)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
use gfx2_extension_runtime::hot_reload_module;

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
}
/*
pub mod hot2 {
    #[doc(hidden)]
    pub mod __load {
        pub struct DllShims<'__lib> {
            fnptr_shorten_lifetime: ::libloading::Symbol<'__lib, *const ::std::ffi::c_void>,
            fnptr_push: ::libloading::Symbol<'__lib, *const ::std::ffi::c_void>,
            fnptr_simple: ::libloading::Symbol<'__lib, *const ::std::ffi::c_void>,
        }
        impl<'__lib> DllShims<'__lib> {
            pub fn shorten_lifetime<'a, 'b, 'min>(&self, arg0: &'a i32, arg1: &'b i32) -> &'min i32
            where
                'a: 'min,
                'b: 'min,
                '__lib: 'a + 'b + 'min,
                &'static i32: gfx2_extension_runtime::DylibSafe
            {
                (unsafe {
                    ::std::mem::transmute::<_, fn(a: &'a i32, b: &'b i32) -> &'min i32>(
                        *self.fnptr_shorten_lifetime,
                    )
                })(arg0, arg1)
            }
            pub fn push<'a, 'b>(&self, arg0: &'a mut Vec<&'b i32>)
            where
                '__lib: 'a + 'b,
            {
                (unsafe {
                    ::std::mem::transmute::<_, fn(v: &'a mut Vec<&'b i32>)>(*self.fnptr_push)
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
                    fnptr_push: unsafe { lib.get(stringify!(push).as_bytes())? },
                    fnptr_simple: unsafe { lib.get(stringify!(simple).as_bytes())? },
                })
            }
        }
    }
}
*/
