//!
//! # Safety of function calls inside dylibs
//!
//! ## Wrapper functions
//! Dynamically loaded functions are exposed as wrapper methods on a macro-generated type.
//! Directly exposing function pointers is possible, but very unsafe: rust assumes that fn pointers
//! are `'static`, thus can be very easily copied and outlive the dylib.
//! The generate wrapper functions have an additional reference parameter (`&self`): this,
//! unfortunately means that the lifetimes in the function signatures passed to the macro cannot
//! be elided (will generate a mismatched lifetime error).
//!
//!
//! ## Lifetime bounds
//! The generated wrapper has bounds on all referenced lifetimes in the function signature,
//! including 'static if it appears. Concretely, there is a bound like:
//! `'lib: <all referenced lifetimes>`
//!
//! The bound can be overly restrictive in some cases: e.g. limiting the lifetime of
//! a returned borrow even if would be valid after the library is unloaded.
//! It can also raise problems with invariant lifetimes, but this is probably not very common,
//! and can be worked around.
//!
//! The fact that we are adding a bound to lifetimes in a macro means that all lifetimes
//! in function declarations in hot-reload modules must appear in the syntax and cannot be elided.
//! An error is generated if lifetimes are elided, but the message is not very clear.
//! The added bound may also lead to unclear error messages in some (most?) cases that point inside
//! the macro.
//!
//!
//! The main remaining issue is that types can contain 'static references to static data.
//! If the `'static` bound appears in the fn signature, then we generate a `'lib: 'static`
//! bound that will prevent the code to compile. But references to static data
//! can be hidden within types, without `'static` appearing in the fn signature.
//! Another overview of this issue is presented in
//! [https://github.com/nagisa/rust_libloading/issues/46].
//!
//!
//!
//! ## Approaches
//!
//! #### Marker traits
//! One possible solution would be a marker trait that says
//! "this type is not hiding &'static references".
//! (or, equivalently, "all references are bound to a lifetime parameter of the type":
//! the idea is that all references contained in the type must be bound to a lifetime parameter
//! so that add bounds to them).
//! Unfortunately, the trait should be implemented manually for all types appearing in signatures,
//! and for all types of std (orphan rules would prevent implementing the trait on the spot).
//! (And additionally, this does not work with type aliases, e.g.: `type A = T<'static>`.
//! Here, T is `DylibSafe` (all inner refs bound to a lifetime parameter) but if we use the alias
//! `A` in the fn signature, the `'static` lifetime is not syntactically present, and we can't
//! add the `'lib: 'static` bound that will prevent compilation.)
//!
//! #### Opt-in traits
//! Currently, with opt-in traits (OIBIT), it is possible to detect if a type contains references
//! ('static or not), and also if the type is 'static. By combining these two pieces of information,
//! we could detect if a type contains 'static references.
//! Unfortunately, this cannot be done with OIBITs right now due to a technical limitation
//! (bounds on type parameters of negative trait impls are not taken into account
//! [https://github.com/rust-lang/rust/issues/23072]; also, equivalently,
//! cannot have negative trait bounds).
//! This should be investigated further, as this would be an elegant approach if it can be made
//! to work somehow.
//!
//! #### Run-time checks
//! In theory, it should be possible to insert code in the wrapper function that scans the arguments
//! and the return values for pointers that fall into the address range of the loaded dylib.
//! This scan will probably have a high runtime overhead (but could be disabled in release mode).
//! It will also require all types in fn signatures to implement a trait that reflects all refs
//! present within the type.
//!
//! #### Keep dylibs in memory
//! Finally, a straightforward solution is to simply never unload dylibs so that the `'static`
//! refs stay valid forever. This removes the need for additional bounds checks and wrapper
//! functions, and greatly simplifies the implementation.
//! It also removes the need for the user to manually un-elide lifetimes in the exported function
//! signatures.
//!
//! Leaking libraries may be acceptable if hot-reloading is only used during the development cycle,
//! but is less desirable in production.
//! A variant of this solution would be to keep the dylibs in memory by default, but unload them
//! if the library is marked as "unload-safe" by the user (e.g. with an (unsafe) attribute like
//! `#[unload_safe]` or something)
//!
//!
//!
//! ## Current solution
//!
//! Warn the user in the documentation about not returning types that hide &'static refs.
//! It cannot be syntactically prevented, and verification through the compiler (with traits)
//! seems intractable at the moment.
//!
//! At some point, may switch to opt-in unloading, and keep dylibs in memory otherwise.
//!
//! New issue: cannot elide constants.
//! A simple 'syntactical un-elision' may work: (&str => &'lib str).
//!
#![feature(fn_traits)]
#![feature(unboxed_closures)]
pub use gfx2_extension_macros::hot_reload_module;
use libloading::Library;
use std::marker::PhantomData;
use std::ops::Deref;

#[derive(Clone)]
#[doc(hidden)]
pub struct FnWrap<'lib, T>(pub T, pub ::std::marker::PhantomData<&'lib ()>);

impl<'lib, T, Args> FnOnce<Args> for FnWrap<'lib, T>
where
    T: FnOnce<Args>,
{
    type Output = <T as FnOnce<Args>>::Output;
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        FnOnce::call_once(self.0, args)
    }
}

impl<'lib, T, Args> FnMut<Args> for FnWrap<'lib, T>
where
    T: FnOnce<Args> + Clone,
{
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        FnOnce::call_once(self.0.clone(), args)
    }
}

impl<'lib, T, Args> Fn<Args> for FnWrap<'lib, T>
where
    T: FnOnce<Args> + Clone,
{
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        FnOnce::call_once(self.0.clone(), args)
    }
}

#[macro_export]
macro_rules! load_module {
    ($lib:expr, $m:path) => {{
        use $m as m;
        m::__load::FnPtrs::load($lib)
    }};
}

#[macro_export]
macro_rules! load_dev_dylib {
    ($crate_name:path) => {{
        use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
        #[cfg(debug_assertions)]
        let subdir = "debug";
        #[cfg(not(debug_assertions))]
        let subdir = "release";
        libloading::Library::new(format!(
            "target/{}/deps/{}{}{}",
            subdir,
            DLL_PREFIX,
            stringify!($crate_name),
            DLL_SUFFIX
        ))
    }};
}
