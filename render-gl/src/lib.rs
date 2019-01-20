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
use autograph_render::traits::Downcast;
use autograph_render::AliasScope;
use std::any::Any;
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

/// Helper for downcasting.
trait DowncastPanic: Downcast {
    fn downcast_ref_unwrap<T: Any>(&self) -> &T {
        self.as_any().downcast_ref().expect("invalid backend type")
    }

    fn downcast_mut_unwrap<T: Any>(&mut self) -> &mut T {
        self.as_any_mut()
            .downcast_mut()
            .expect("invalid backend type")
    }

    fn downcast_unwrap<T: Any>(self: Box<Self>) -> Box<T> {
        self.into_any().downcast().expect("invalid backend type")
    }
}

impl<T: Downcast + ?Sized> DowncastPanic for T {}

//--------------------------------------------------------------------------------------------------
pub type Backend = backend::OpenGlBackend;
