//!
//! The first issue with passing objects across dylibs is that some objects
//! own heap allocated data and cannot be safely transferred between dylibs.
//!
//! The other issue is that objects may contain references to static data inside the dylib
//! that become invalid as soon as the dylib is unloaded.
//! Currently, it is practically impossible to statically check if an object contains a reference
//! to static data.
//! Instead, we can do this check at runtime, by enumerating all pointers inside the value and
//! checking if they falls in the address range of the dylib.
//!
//! Limitation: not only return values: can mutate a container, need to check what we put inside.
//!
//!
pub use gfx2_extension_macros::hot_reload_module;

pub unsafe trait DylibSend
{
    fn reflect_ptrs(&self, v: &BorrowLeakVerifier);
}

pub struct BorrowLeakVerifier;

impl BorrowLeakVerifier {
    fn visit_ptr<T>(&self, ptr: *const T) {
        // no-op for now
    }
}

#[macro_export]
macro_rules! load_module {
    ($lib:expr, $m:path) => {
        {
            use $m as m;
            m::__load::FnPtrs::load($lib)
        }
    };
}
