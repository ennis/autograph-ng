use super::shader_interface::{
    InterfaceBinder, ShaderInterface, ShaderInterfaceDesc, TypeDesc, VertexLayout,
};
use failure::Error;
use gfx::{BufferSliceAny, Format, Sampler, TextureAny};
use gl;
use gl::types::*;

/// A trait representing a shader
pub trait Shader {}
pub trait VertexShader: Shader {}
pub trait FragmentShader: Shader {}
pub trait GeometryShader: Shader {}
pub trait TessControlShader: Shader {}
pub trait TessEvalShader: Shader {}
pub trait ComputeShader: Shader {}

/// A trait representing a collection of shaders (of the same type)
/// for the whole graphics pipeline.
/// Provides methods for binding data to the OpenGL pipeline.
///
/// Potential implementations: GlslShaderPipeline, GlslBinaryPipeline, SpirvBinaryPipeline, etc.
pub trait GraphicsShaderPipeline {
    fn vertex_shader(&self) -> &VertexShader;
    fn fragment_shader(&self) -> &FragmentShader;
    fn geometry_shader(&self) -> Option<&GeometryShader>;
    fn tess_control_shader(&self) -> Option<&TessControlShader>;
    fn tess_eval_shader(&self) -> Option<&TessEvalShader>;
    fn is_compatible_with(&self, interface: &ShaderInterfaceDesc) -> Result<(), Error>;
    fn get_program(&self) -> Result<GLuint, Error>;
}

pub trait ComputeShaderPipeline {
    fn compute_shader(&self) -> &ComputeShader;
    fn is_compatible_with(&self, interface: &ShaderInterfaceDesc) -> Result<(), Error>;
    fn get_program(&self) -> Result<GLuint, Error>;
}
