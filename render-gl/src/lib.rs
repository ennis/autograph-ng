//! OpenGL backend for autograph-render.
//!
//!
//! ### Images
//!
//! Images that are used exclusively as render targets (i.e. images that only have the COLOR_ATTACHMENT
//! usage flag set) are created as OpenGL renderbuffers. If any other usage flag is set, a regular
//! texture is allocated instead.
//!
//! ### Presentation
//!
//! Currently, it's not possible to render directly into the default framebuffer: all rendering
//! operations must be done into a texture.
//! The "present" command then copies the specified image to the default framebuffer with
//! `glBlitFramebuffer`, and then calls `SwapBuffers`.
//!
//! ### Texture & viewport coordinates
//!
//! OpenGL sets the origin of viewports and textures to the lower-left corner. For clip-space,
//! the lower-left corner is at coordinates (-1,1).
//! For texture data, this means that OpenGL expects the first scanline to be the lowest row of
//! pixels in the image.
//!
//! For consistency with other backends, all texture data is stored upside-down:
//! i.e. the first scanline will actually be the topmost row of pixels in the original image.
//! As with other backends, texcoord (0,0) will sample the upper-left pixel, and
//! when rendering to a texture, the (-1,-1) coordinate in clip space will map to the upper-left corner.
//!
//! This is contrary to the usual convention of OpenGL: in debuggers such as RenderDoc,
//! the contents of textures and render targets will appear to be flipped vertically.
//!
//! In order for the images to appear correctly on the screen, image data is flipped vertically
//! before a "present" operation.
//!
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
pub mod prelude;
mod sampler;
mod swapchain;
mod sync;
mod util;
mod window;

pub use self::{
    backend::{InstanceConfig, OpenGlBackend, OpenGlInstance},
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
