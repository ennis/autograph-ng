use crate::backend;

//--------------------------------------------------------------------------------------------------
pub type Backend = backend::OpenGlBackend;
pub type Arena<'a> = autograph_api::Arena<'a, Backend>;
pub type Buffer<'a, T> = autograph_api::buffer::Buffer<'a, Backend, T>;
pub type TypedConstantBufferView<'a, T> =
    autograph_api::buffer::TypedConstantBufferView<'a, Backend, T>;
pub type Image2d<'a> = autograph_api::image::Image2d<'a, Backend>;
pub type RenderTarget2dView<'a> = autograph_api::image::RenderTarget2dView<'a, Backend>;
pub type TextureSampler2dView<'a> = autograph_api::image::TextureSampler2dView<'a, Backend>;
pub type TypedGraphicsPipeline<'a, T> =
    autograph_api::pipeline::TypedGraphicsPipeline<'a, Backend, T>;
pub type TypedArgumentBlock<'a, T> = autograph_api::pipeline::TypedArgumentBlock<'a, Backend, T>;
pub type DynamicSignature<'a> = autograph_api::pipeline::DynamicSignature<'a, Backend>;
