#![feature(align_offset)]
#[macro_use]
extern crate log;

mod aliaspool;
mod api;
mod backend;
mod buffer;
mod command;
mod format;
mod framebuffer;
mod image;
mod pipeline;
mod sampler;
mod swapchain;
mod sync;
mod util;
mod window;

pub use self::{
    backend::{OpenGlBackend, OpenGlInstance},
    window::create_instance_and_window,
};

use crate::api as gl;
use autograph_render::AliasScope;
use std::mem;

#[derive(Copy, Clone, Debug)]
struct AliasInfo<K: slotmap::Key> {
    key: K,
    scope: AliasScope,
}

//--------------------------------------------------------------------------------------------------
pub struct ImplementationParameters {
    pub uniform_buffer_alignment: usize,
    pub max_draw_buffers: u32,
    pub max_color_attachments: u32,
    pub max_viewports: u32,
}

impl ImplementationParameters {
    pub fn populate(gl: &gl::Gl) -> ImplementationParameters {
        let getint = |param| unsafe {
            let mut v = mem::uninitialized();
            gl.GetIntegerv(param, &mut v);
            v
        };

        ImplementationParameters {
            uniform_buffer_alignment: getint(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT) as usize,
            max_draw_buffers: getint(gl::MAX_DRAW_BUFFERS) as u32,
            max_color_attachments: getint(gl::MAX_COLOR_ATTACHMENTS) as u32,
            max_viewports: getint(gl::MAX_VIEWPORTS) as u32,
        }
    }
}

//--------------------------------------------------------------------------------------------------
pub type Backend = backend::OpenGlBackend;
