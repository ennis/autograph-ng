use failure::Error;

use super::bind::StateCache;
use super::buffer_data::BufferData;
use super::format::Format;
use super::pipeline::GraphicsPipeline;
use super::texture::*;
use super::upload_buffer::UploadBuffer;
use super::{Frame, ResourceTracker};


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    Int,
    UnsignedInt,
    Half, //?
    Float,
    Double,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// GLSL/SPIR-V types used to interface with shader programs.
/// i.e. the types used to describe a buffer interface.
///
pub enum TypeDesc {
    Primitive(PrimitiveType),
    /// Array type, may have special alignment constraints
    Array(Box<TypeDesc>, usize),
    /// Vector type (ty,size), not all sizes are valid.
    Vector(PrimitiveType, u8),
    /// Matrix type (ty,rows,cols), not all combinations of rows and cols are valid.
    Matrix(PrimitiveType, u8, u8),
    /// A structure type: (offset, typedesc)
    Struct(Vec<(usize, TypeDesc)>),
    Unknown,
}

pub const TYPE_FLOAT: TypeDesc = TypeDesc::Primitive(PrimitiveType::Float);
pub const TYPE_INT: TypeDesc = TypeDesc::Primitive(PrimitiveType::Int);
pub const TYPE_VEC2: TypeDesc = TypeDesc::Vector(PrimitiveType::Float, 2);
pub const TYPE_VEC3: TypeDesc = TypeDesc::Vector(PrimitiveType::Float, 3);
pub const TYPE_VEC4: TypeDesc = TypeDesc::Vector(PrimitiveType::Float, 4);
pub const TYPE_IVEC2: TypeDesc = TypeDesc::Vector(PrimitiveType::Int, 2);
pub const TYPE_IVEC3: TypeDesc = TypeDesc::Vector(PrimitiveType::Int, 3);
pub const TYPE_IVEC4: TypeDesc = TypeDesc::Vector(PrimitiveType::Int, 4);
pub const TYPE_MAT2: TypeDesc = TypeDesc::Matrix(PrimitiveType::Float, 2, 2);
pub const TYPE_MAT3: TypeDesc = TypeDesc::Matrix(PrimitiveType::Float, 3, 3);
pub const TYPE_MAT4: TypeDesc = TypeDesc::Matrix(PrimitiveType::Float, 4, 4);

// vertex type: interpretation (FLOAT,UNORM,SNORM,INTEGER)

/// Describes a render target binding (a framebuffer attachement, in GL parlance)
#[derive(Clone, Debug)]
pub struct RenderTargetDesc {
    pub name: Option<String>,
    pub index: Option<u32>,
    pub format: Option<Format>,
}

#[derive(Clone, Debug)]
pub struct UniformConstantDesc {
    pub name: Option<String>,
    pub index: Option<u32>,
    pub ty: &'static TypeDesc,
}

/// An uniform buffer
#[derive(Clone, Debug)]
pub struct UniformBufferDesc {
    pub name: Option<String>,
    pub index: Option<u32>,
    /// This can be none if using a BufferSliceAny
    pub tydesc: Option<&'static TypeDesc>,
}

/// An input buffer for vertex data
#[derive(Clone, Debug)]
pub struct VertexBufferDesc {
    pub name: Option<String>,
    pub index: Option<u32>,
    pub layout: &'static VertexLayout,
}

/// An input buffer for indices
#[derive(Clone, Debug)]
pub struct IndexBufferDesc {
    pub format: Format,
}

/// Texture basic data type (NOT storage format)
#[derive(Copy, Clone, Debug)]
pub enum TextureDataType {
    Float, // and also depth
    Integer,
    UnsignedInteger,
}

/// Represents a texture binding expected by a shader.
/// The shader should not specify a particular storage format but rather the component type
/// of the texture elements (either float, integer, or unsigned)
#[derive(Clone, Debug)]
pub struct TextureBindingDesc {
    /// name of the texture binding in shader, can be None.
    /// name and index should not be both None
    pub name: Option<String>,
    /// index (texture uint) of the texture binding in shader, can be None.
    /// name and index should not be both None
    pub index: Option<u32>,
    /// Basic data type of the texture (if known)
    pub data_type: Option<TextureDataType>,
    /// dimensions (if known)
    pub dimensions: Option<TextureDimensions>,
}

/// A trait defined for types that can be bound to the pipeline as an image.
pub trait TextureInterface: Into<TextureAny> + 'static {
    fn get_data_type() -> Option<TextureDataType>;
    fn get_dimensions() -> Option<TextureDimensions>;
}

impl TextureInterface for Texture2D {
    fn get_data_type() -> Option<TextureDataType> {
        None
    }
    fn get_dimensions() -> Option<TextureDimensions> {
        Some(TextureDimensions::Tex2D)
    }
}

impl TextureInterface for TextureAny {
    fn get_data_type() -> Option<TextureDataType> {
        None
    }
    fn get_dimensions() -> Option<TextureDimensions> {
        None
    }
}

pub trait SampledTextureInterface {
    type TextureType: TextureInterface;
    //fn get_texture(&self) -> &Self::TextureType;
    fn get_sampler(&self) -> &gfx::SamplerDesc;
    fn into_texture_any(self) -> TextureAny;
}

impl SampledTextureInterface for SampledTexture2D {
    type TextureType = Texture2D;
    fn into_texture_any(self) -> TextureAny {
        self.0.into()
    }
    fn get_sampler(&self) -> &gfx::SamplerDesc {
        &self.1
    }
}

/// Trait implemented by types that can serve as a vertex attribute.
pub unsafe trait VertexAttributeType {
    /// The equivalent OpenGL type (the type seen by the shader).
    const EQUIVALENT_TYPE: TypeDesc;
    /// Returns the corresponding data format (the layout of the data in memory).
    const FORMAT: Format;
}

macro_rules! impl_vertex_attrib_type {
    ($t:ty, $equiv:expr, $fmt:ident) => {
        unsafe impl VertexAttributeType for $t {
            const EQUIVALENT_TYPE: TypeDesc = $equiv;
            const FORMAT: Format = Format::$fmt;
        }
    };
}

impl_vertex_attrib_type!(f32, TypeDesc::Primitive(PrimitiveType::Float), R32_SFLOAT);
impl_vertex_attrib_type!(
    [f32; 2],
    TypeDesc::Vector(PrimitiveType::Float, 2),
    R32G32_SFLOAT
);
impl_vertex_attrib_type!(
    [f32; 3],
    TypeDesc::Vector(PrimitiveType::Float, 3),
    R32G32B32_SFLOAT
);
impl_vertex_attrib_type!(
    [f32; 4],
    TypeDesc::Vector(PrimitiveType::Float, 4),
    R32G32B32A32_SFLOAT
);

/// Trait implemented by types that can serve as indices.
pub unsafe trait IndexElementType: BufferData {
    /// Returns the corresponding data format (the layout of the data in memory).
    const FORMAT: Format;
}

macro_rules! impl_index_element_type {
    ($t:ty, $fmt:ident) => {
        unsafe impl IndexElementType for $t {
            const FORMAT: Format = Format::$fmt;
        }
    };
}

impl_index_element_type!(u16, R16_UINT);
impl_index_element_type!(u32, R32_UINT);

/// Trait implemented by types that are layout-compatible with an specific
/// to GLSL/SPIR-V type.
/// An implementation is provided for most primitive types and arrays of primitive types.
/// Structs can derive it automatically with `#[derive(BufferLayout)]`
pub unsafe trait BufferLayout {
    fn get_description() -> &'static TypeDesc;
}

/// Trait implemented by types that can be bound to the pipeline with a
/// variant of glProgramUniform
/// An implementation is provided for most primitive types .
pub unsafe trait UniformInterface {
    fn get_description() -> &'static TypeDesc;
}

macro_rules! impl_uniform_type {
    ($t:ty, $tydesc:expr) => {
        unsafe impl BufferLayout for $t {
            fn get_description() -> &'static TypeDesc {
                static DESC: TypeDesc = $tydesc;
                &DESC
            }
        }
        unsafe impl UniformInterface for $t {
            fn get_description() -> &'static TypeDesc {
                static DESC: TypeDesc = $tydesc;
                &DESC
            }
        }
    };
}

impl_uniform_type!(f32, TypeDesc::Primitive(PrimitiveType::Float));
impl_uniform_type!([f32; 2], TypeDesc::Vector(PrimitiveType::Float, 2));
impl_uniform_type!([f32; 3], TypeDesc::Vector(PrimitiveType::Float, 3));
impl_uniform_type!([f32; 4], TypeDesc::Vector(PrimitiveType::Float, 4));
impl_uniform_type!(i32, TypeDesc::Primitive(PrimitiveType::Int));
impl_uniform_type!([i32; 2], TypeDesc::Vector(PrimitiveType::Int, 2));
impl_uniform_type!([i32; 3], TypeDesc::Vector(PrimitiveType::Int, 3));
impl_uniform_type!([i32; 4], TypeDesc::Vector(PrimitiveType::Int, 4));
impl_uniform_type!([[f32; 2]; 2], TypeDesc::Matrix(PrimitiveType::Float, 2, 2));
impl_uniform_type!([[f32; 3]; 3], TypeDesc::Matrix(PrimitiveType::Float, 3, 3));
impl_uniform_type!([[f32; 4]; 4], TypeDesc::Matrix(PrimitiveType::Float, 4, 4));

/// Trait implemented by types that can be bound to the pipeline as a buffer object
pub unsafe trait BufferInterface: gfx::ToBufferSliceAny {
    /// Get the layout of the buffer data, if it is known.
    fn get_layout() -> Option<&'static TypeDesc>;
}

/*unsafe impl<T: BufferData+BufferLayout> BufferInterface for gfx::Buffer<T>
{
    fn get_layout() -> Option<&'static BufferLayout> {
        Some(<T as BufferLayout>::get_description())
    }
}

unsafe impl BufferInterface for gfx::BufferAny
{
    fn get_layout() -> Option<&'static BufferLayout> {
        None
    }
}*/

// impl for typed buffers
unsafe impl<T: BufferData + BufferLayout> BufferInterface for gfx::BufferSlice<T> {
    fn get_layout() -> Option<&'static TypeDesc> {
        Some(<T as BufferLayout>::get_description())
    }
}

// impl for untyped buffers
unsafe impl BufferInterface for gfx::BufferSliceAny {
    fn get_layout() -> Option<&'static TypeDesc> {
        None
    }
}

/// Description of a vertex attribute.
#[derive(Clone, Debug)]
pub struct VertexAttributeDesc {
    /// Attribute name.
    pub name: Option<String>,
    /// Location.
    pub loc: u8,
    /// The equivalent OpenGL type.
    pub ty: TypeDesc,
    /// Storage format of the vertex attribute.
    pub format: Format,
    /// Relative offset.
    pub offset: u8,
}

/// The layout of vertex data in a vertex buffer.
#[derive(Clone, Debug)]
pub struct VertexLayout {
    pub attributes: &'static [VertexAttributeDesc],
    pub stride: usize,
}

///
/// Trait implemented by types that represent vertex data in a vertex buffer.
/// This is used to automatically infer the vertex layout.
///
/// ```rust
/// #[derive(VertexType)]
/// #[repr(C)]
/// struct MyVertexType {
///     position: Vec3,
///     normals: Vec3,
///     tangents: Vec3,
///     texcoords: Vec2,
/// }
/// ```
pub trait VertexType: BufferData {
    fn get_layout() -> &'static VertexLayout;
}

/// Descriptions of shader interfaces.
///
/// This trait is a facade to recover information about the bindings defined in a shader interface.
/// It is meant to be derived automatically with `#[derive(ShaderInterface)]`, but you can implement it by hand.
///
/// TODO replace it with a simple struct?
/// TODO reduce the number of members
pub trait ShaderInterfaceDesc: Sync + 'static {
    /// Returns the list of uniform buffers (`#[uniform_buffer]`)
    fn get_uniform_buffers(&self) -> &[UniformBufferDesc];
    /// Returns the list of named uniform items (`#[uniform_constant]`)
    fn get_uniform_constants(&self) -> &[UniformConstantDesc];
    /// Returns the list of render target items (`#[render_target(...)]`)
    fn get_render_targets(&self) -> &[RenderTargetDesc];
    /// Returns the list of vertex buffer items (`#[vertex_buffer(index=...)]`)
    fn get_vertex_buffers(&self) -> &[VertexBufferDesc];
    /// Returns the index buffer item, if any (`#[index_buffer]`)
    fn get_index_buffer(&self) -> Option<&IndexBufferDesc>;
    /// Returns the list of texture/sampler pairs (`#[texture_binding(index=...,data_type=...)]`)
    fn get_texture_bindings(&self) -> &[TextureBindingDesc];
}

pub struct InterfaceBindingContext<'a> {
    pub gctx: &'a gfx::Context,
    pub tracker: &'a mut ResourceTracker,
    pub upload_buffer: &'a UploadBuffer,
    pub state_cache: &'a mut StateCache,
}

pub trait InterfaceBinder<T: ShaderInterface> {
    /// Binds the contents of the shader interface to the OpenGL pipeline, without any validation.
    /// Validation is intended to be done when creating the graphics/compute pipeline.
    ///
    /// uniform constant => <ty as UniformConstantInterface>.bind(uniform_binder);
    /// uniform buffer => uniform_binder.bind(binding, buffer)
    unsafe fn bind_unchecked(&self, interface: &T, frame: &Frame, state_cache: &mut StateCache);
}

/// Trait implemented by types that represent a shader interface.
///
/// A shader interface is a set of uniforms, vertex attributes, render targets
/// that describe the inputs and outputs of a graphics pipeline.
///
/// ```rust
/// #[derive(ShaderInterface)]
/// #[shader_interface(deny_incomplete)]
/// struct MyShaderInterface {
///     #[vertex_buffer(index=0)]
///     vbuf: VertexBuffer<MyVertexType>,    // a type-safe wrapper around a buffer
///     #[index_buffer]
///     ibuf: IndexBuffer<u16>,
///     #[texture(index=0)],
///     albedo: FloatTexture,
///     #[texture(index=1)],
///     specularMap: FloatTexture
/// }
/// ```
pub trait ShaderInterface {
    fn get_description() -> &'static ShaderInterfaceDesc;
    /// Creates an _interface binder_ object that will handle binding interfaces of this specific
    /// type to the OpenGL pipeline.
    ///
    /// Returns an error if the shader interface does not match with the given pipeline.
    fn create_interface_binder(
        pipeline: &GraphicsPipeline,
    ) -> Result<Box<InterfaceBinder<Self>>, Error>
    where
        Self: Sized;
}

//
// pipeline.into_typed::<T: ShaderInterface>()
//      let binder = <T as ShaderInterface>::create_interface_binder(self)
//      TypedGraphicsPipeline { orig: pipeline.clone(), binder }
//
// draw()
//   typed_pipeline.bind()
//      pipeline.bind()
//      interface_binder.bind(binder)
//
