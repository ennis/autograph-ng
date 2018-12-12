//#![feature(rust_2018_preview, uniform_paths)]
//#![feature(vec_remove_item)]
//#![feature(arbitrary_self_types)]
#![feature(const_transmute)]

extern crate typed_arena;
extern crate unreachable;
#[macro_use]
extern crate bitflags;
extern crate config;
extern crate regex;
extern crate toml;
#[macro_use]
extern crate lazy_static;
//#[macro_use]
//extern crate ash;
extern crate pretty_env_logger;
//#[cfg(target_os = "windows")]
//extern crate winapi;
extern crate glutin;
extern crate winit;
#[macro_use]
extern crate log;
#[macro_use]
extern crate slotmap;
extern crate fxhash;
extern crate ordered_float;
extern crate shaderc;
extern crate smallvec;
extern crate time;
#[macro_use]
extern crate derivative;

pub mod app;
pub mod renderer;

//pub use self::app::*;
