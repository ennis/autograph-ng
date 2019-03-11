use crate::backend;

//--------------------------------------------------------------------------------------------------
pub type Backend = backend::OpenGlBackend;
pub type Arena<'a> = autograph_render::Arena<'a, Backend>;
pub type Buffer<'a, T> = autograph_render::buffer::Buffer<'a, Backend, T>;
pub type TypedConstantBufferView<'a, T> =
    autograph_render::buffer::TypedConstantBufferView<'a, Backend, T>;
pub type Image2d<'a> = autograph_render::image::Image2d<'a, Backend>;
pub type RenderTarget2dView<'a> = autograph_render::image::RenderTarget2dView<'a, Backend>;
pub type TextureSampler2dView<'a> = autograph_render::image::TextureSampler2dView<'a, Backend>;
pub type TypedGraphicsPipeline<'a, T> =
    autograph_render::pipeline::TypedGraphicsPipeline<'a, Backend, T>;
pub type TypedArgumentBlock<'a, T> = autograph_render::pipeline::TypedArgumentBlock<'a, Backend, T>;
