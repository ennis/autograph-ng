#![feature(proc_macro_hygiene)]
use gfx2_shader::*;

#[no_mangle]
pub static BLIT_SHADERS: CombinedShaders = include_combined_shader!("tests/data/shaders/blit.glsl");

#[no_mangle]
pub extern fn plugin_entry() {
    println!("loaded plugin");
}
