//#![feature(rust_2018_preview, uniform_paths)]
#![feature(vec_remove_item)]
extern crate petgraph;
#[macro_use]
extern crate bitflags;
extern crate config;
extern crate toml;
#[macro_use]
extern crate ash;
extern crate pretty_env_logger;
#[cfg(target_os = "windows")]
extern crate winapi;
extern crate winit;
#[macro_use]
extern crate log;
extern crate serde;
extern crate slotmap;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate downcast_rs;
extern crate sid_vec;
extern crate time;

pub mod alloc;
mod buffer_data;
pub mod context;
pub mod frame;
mod handle;
pub mod import;
pub mod resource;
mod sync;
pub mod window;

// re-export vulkan as gfx2::vk
pub use ash::vk;

pub use self::context::*;
pub use self::frame::*;
pub use self::resource::*;
pub use self::window::*;
