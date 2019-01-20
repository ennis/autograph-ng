#![allow(clippy::all)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

unsafe impl Sync for Gl {}
