pub use gfx2_extension_macros::hot_reload_module;

#[macro_export]
macro_rules! load_module {
    ($lib:expr, $m:path) => {
        {
            use $m as m;
            m::__load::DllShims::load($lib)
        }
    };
}
