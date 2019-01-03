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
//!
//! ## Current solution
//!
//! Warn the user in the documentation about not returning types that hide &'static refs.
//! It cannot be syntactically prevented, and verification through the compiler (with traits)
//! seems intractable at the moment.
//!
//! At some point, may switch to opt-in unloading, and keep dylibs in memory otherwise.
//!
#![feature(optin_builtin_traits)]
#![feature(on_unimplemented)]
pub use gfx2_extension_macros::hot_reload_module;
use std::marker::PhantomData;
use std::cell::Cell;

//pub unsafe auto trait DylibSafe {}

#[rustc_on_unimplemented(
    message = "`{Self}` cannot be moved across hot-reloadable libraries safely",
    label = "`{Self}` cannot be moved across hot-reloadable libraries safely"
)]
pub trait DylibSafe {}

impl DylibSafe for u8 {}
impl DylibSafe for u16 {}
impl DylibSafe for u32 {}
impl DylibSafe for u64 {}
impl DylibSafe for u128 {}
impl DylibSafe for i8 {}
impl DylibSafe for i16 {}
impl DylibSafe for i32 {}
impl DylibSafe for i64 {}
impl DylibSafe for i128 {}
impl DylibSafe for f32 {}
impl DylibSafe for f64 {}
impl DylibSafe for char {}
impl DylibSafe for str {}
impl<T: DylibSafe> DylibSafe for [T] {}

// impl for references
// note: this surprisingly says that &'static is DylibSafe, but this is not a problem, as long
// as the lifetime is syntactically present in the spelled-out type.
//
// XXX: what about type aliases?
// type A = &'static i32;   // welp.
// A is DylibSafe, but 'static is not syntactically present in the function signature
// -> Conclusion: this cannot work

// tuples
macro_rules! dylibsafe_tuple_impl {
    ($($t:ident),*) => {
        impl<$($t: DylibSafe),*> DylibSafe for ($($t,)*) {}
    };
}

//dylibsafe_tuple_impl!{T0}
dylibsafe_tuple_impl!{T0,T1}
dylibsafe_tuple_impl!{T0,T1,T2}
dylibsafe_tuple_impl!{T0,T1,T2,T3}
dylibsafe_tuple_impl!{T0,T1,T2,T3,T4}
dylibsafe_tuple_impl!{T0,T1,T2,T3,T4,T5}
dylibsafe_tuple_impl!{T0,T1,T2,T3,T4,T5,T6}
dylibsafe_tuple_impl!{T0,T1,T2,T3,T4,T5,T6,T7}
dylibsafe_tuple_impl!{T0,T1,T2,T3,T4,T5,T6,T7,T8}
dylibsafe_tuple_impl!{T0,T1,T2,T3,T4,T5,T6,T7,T8,T9}
dylibsafe_tuple_impl!{T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10}
dylibsafe_tuple_impl!{T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11}

// std containers
impl<T: DylibSafe> DylibSafe for Vec<T> {}


/*impl<T> DylibSafe for T where Cell<T>: DylibSafe0 {}
pub unsafe auto trait DylibSafe0 {}
impl<T: ?Sized> !DylibSafe for Cell<&'static T> {}
impl<T: ?Sized> !DylibSafe for Cell<&'static mut T> {}
*/

// the thing is that &'static T <: &'a T
// Fix lifetimes somehow?

// see if there is an impl of DylibSafe for &'a T, for all 'a
// -> negative impl for 'a = 'static
// -> disables default impl
//
// -> can detect if something contains a ref
// -> can detect if something is 'static
// -> if something contains a ref and is provably 'static => fail!
// -> or impl if NoRefs OR not 'static
// -> HasReferences + 'static (or !HasReferences OR not static)
//
// DylibSafe if NoRefs
// OR not 'static
// unimpl if HasRefs
//
// T: 'a (may be 'static)
// -> goal: have 'a inferred to be the longest lifetime possible, independently of the 'lib: 'a bound
// if T is U<'b> then 'a = 'b, else T: 'static
// output bounded by same 'a
// and 'lib: 'a


#[macro_export]
macro_rules! load_module {
    ($lib:expr, $m:path) => {{
        use $m as m;
        m::__load::DllShims::load($lib)
    }};
}
