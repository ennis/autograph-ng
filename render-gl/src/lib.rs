#![feature(align_offset)]
#[macro_use]
extern crate log;

mod aliaspool;
mod api;
mod backend;
mod buffer;
mod command;
mod descriptor;
mod format;
mod framebuffer;
mod image;
mod pipeline;
mod sampler;
mod swapchain;
mod sync;
mod util;
mod window;

pub use self::swapchain::SwapchainInner;
pub use self::window::create_backend_and_window;

use crate::api as gl;
use autograph_render::AliasScope;
use std::mem;
use autograph_render::handle;

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

unsafe trait HandleCast<'a, T> {
    unsafe fn cast(self) -> &'a T;
}

macro_rules! impl_handle {
    ($handle:ident, $impl_ty:ty) => {
        unsafe impl<'a> HandleCast<'a, $impl_ty> for handle::$handle<'a> {
            unsafe fn cast(self) -> &'a $impl_ty {
                &*(self.0 as *const $impl_ty)
            }
        }

        impl<'a> From<&'a $impl_ty> for handle::$handle<'a> {
            fn from(v: &'a $impl_ty) -> handle::$handle<'a> {
                handle::$handle(v as *const _ as usize, std::marker::PhantomData)
            }
        }

        impl<'a> From<&'a mut $impl_ty> for handle::$handle<'a> {
            fn from(v: &'a mut $impl_ty) -> handle::$handle<'a> {
                handle::$handle(v as *const _ as usize, std::marker::PhantomData)
            }
        }
    };
}

impl_handle!(Image, image::GlImage);
impl_handle!(Buffer, buffer::GlBuffer);
impl_handle!(ShaderModule, pipeline::GlShaderModule);
impl_handle!(GraphicsPipeline, pipeline::GlGraphicsPipeline);
impl_handle!(Swapchain, swapchain::GlSwapchain);
impl_handle!(PipelineSignature, pipeline::GlPipelineSignature<'a>);
impl_handle!(PipelineArguments, pipeline::GlPipelineArguments<'a>);
impl_handle!(Arena, backend::GlArena);


//--------------------------------------------------------------------------------------------------
pub type Backend = backend::OpenGlBackend;
