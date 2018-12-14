#![feature(const_transmute)]

#[macro_use]
extern crate log;

// Reexport nalgebra_glm types if requested
#[cfg(feature = "glm-types")]
pub use nalgebra_glm as glm;

pub mod app;
pub mod renderer;
