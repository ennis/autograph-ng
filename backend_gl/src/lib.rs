#![feature(duration_as_u128)]
#[macro_use]
extern crate log;

mod api;
mod backend;
mod buffer;
mod cmd;
mod descriptor;
mod format;
mod framebuffer;
mod image;
mod pipeline;
pub mod pipeline_file;
mod pool;
mod resource;
mod shader;
mod state;
mod sync;
mod upload;
mod util;
mod window;

use gfx2;

pub use self::backend::OpenGlBackend;
pub use self::pipeline_file::PipelineDescriptionFile;
pub use self::window::create_backend_and_window;

pub type Backend = OpenGlBackend;
pub type Buffer<'a, T> = gfx2::Buffer<'a, OpenGlBackend, T>;
pub type BufferTypeless<'a> = gfx2::BufferTypeless<'a, OpenGlBackend>;
pub type Image<'a> = gfx2::Image<'a, OpenGlBackend>;
pub type Framebuffer<'a> = gfx2::Framebuffer<'a, OpenGlBackend>;
pub type DescriptorSet<'a> = gfx2::DescriptorSet<'a, OpenGlBackend>;
pub type DescriptorSetLayout<'a> = gfx2::DescriptorSetLayout<'a, OpenGlBackend>;
pub type GraphicsPipeline<'a> = gfx2::GraphicsPipeline<'a, OpenGlBackend>;
pub type Arena<'a> = gfx2::Arena<'a, OpenGlBackend>;
