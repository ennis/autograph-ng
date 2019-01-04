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
pub use gfx2_extension_macros::hot_reload_module;
use libloading::{Library, Symbol};
use log::debug;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time;
use std::sync::mpsc::{channel, Receiver};
use notify::{Watcher, DebouncedEvent, RecommendedWatcher};

pub struct Dylib {
    tmppath: PathBuf,
    libname: String,    // for logging only
    lib: Option<Library>,
    events: Receiver<DebouncedEvent>,
    _watcher: RecommendedWatcher
}

impl Dylib {
    /// Copy the dylib in a temporary location before loading (some OSes lock the library file
    /// while it's loaded)
    pub fn copy_and_load<P: AsRef<Path>>(libpath: P) -> std::io::Result<Dylib> {
        let mut tmppath = env::temp_dir();
        let libname = libpath
            .as_ref()
            .file_name()
            .ok_or(std::io::ErrorKind::NotFound)?
            .to_str()
            .expect("lib name was not valid UTF-8");
        // generate a unique id
        // taken from https://github.com/irh/rust-hot-reloading
        let unique_name = {
            let timestamp = time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let index = libname.rfind('.').unwrap_or(libname.len());
            let (before, after) = libname.split_at(index);
            format!("{}-{}{}", before, timestamp, after)
        };
        tmppath.push(unique_name);
        debug!(
            "copying dylib: {} -> {}",
            libpath.as_ref().display(),
            tmppath.display()
        );
        // copy file
        fs::copy(libpath.as_ref(), &tmppath)?;
        // crate watcher
        let (tx, rx) = channel();
        let mut watcher = notify::watcher(tx, time::Duration::from_secs(1)).expect("failed to create watcher");
        watcher.watch(libpath.as_ref(), notify::RecursiveMode::NonRecursive).expect("failed to watch library");

        // load library
        let lib = Library::new(&tmppath)?;

        Ok(Dylib {
            lib: lib.into(),
            tmppath,
            libname: libname.to_string(),
            events: rx,
            _watcher: watcher
        })
    }

    pub unsafe fn get<T>(&self, symname: &str) -> std::io::Result<Symbol<T>> {
        self.lib.as_ref().unwrap().get(symname.as_bytes())
    }

    pub fn should_reload(&self) -> bool {
        self.events.try_iter().any(|ev| match ev {
            DebouncedEvent::Write(_) => {
                debug!("detected write on {}", self.libname);
                true
            },
            _ => false
        })
    }
}

impl Drop for Dylib {
    fn drop(&mut self) {
        // force lib to drop, as otherwise the file may still be locked
        self.lib = None;
        fs::remove_file(&self.tmppath).unwrap_or_else(|_| {
            panic!(
                "failed to delete temporary library file at {}",
                self.tmppath.display()
            )
        });
    }
}

#[macro_export]
macro_rules! load_module {
    ($lib:expr, $m:path) => {{
        use $m as m;
        m::__load::DllShims::load($lib)
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
        $crate::Dylib::copy_and_load(&format!(
            "target/{}/deps/{}{}{}",
            subdir,
            DLL_PREFIX,
            stringify!($crate_name),
            DLL_SUFFIX
        ))
    }};
}
