use crate::renderer::format::Format;
use crate::renderer::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    Int,
    UnsignedInt,
    Half, //?
    Float,
    Double,
}

/// Texture basic data type (NOT storage format)
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ImageDataType {
    Float, // and also depth
    Integer,
    UnsignedInteger,
}

/// GLSL/SPIR-V types used to interface with shader programs.
/// i.e. the types used to describe a buffer interface.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TypeDesc<'tcx> {
    Primitive(PrimitiveType),
    /// Array type, may have special alignment constraints
    Array(&'tcx TypeDesc<'tcx>, usize),
    /// Vector type (ty,size), not all sizes are valid.
    Vector(PrimitiveType, u8),
    /// Matrix type (ty,rows,cols), not all combinations of rows and cols are valid.
    Matrix(PrimitiveType, u8, u8),
    /// A structure type: (offset, typedesc)
    Struct(&'tcx [(usize, TypeDesc<'tcx>)]),
    /// An image type.
    Image(ImageDataType, Option<Format>),
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

#[derive(Copy, Clone, Debug)]
pub struct DescriptorSetDescription<'tcx> {
    pub descriptors: &'tcx [DescriptorSetLayoutBinding<'tcx>],
}

pub trait DescriptorSetInterfaceVisitor<'a, R: RendererBackend> {
    fn visit_buffer(&mut self, binding: u32, buffer: &'a R::Buffer);
    //fn visit_sampled_image(&self, binding: u32, image: ImageHandle, sampler: SamplerDescriptor);
    //fn visit_vertex_input<'a>(&self, buffer: &'a R::Buffer);
    //fn visit_fragment_output<'a>(&self, image: &'a R::Image);
    //fn visit_data(&self, binding: u32, data: &[u8]);
}

pub trait DescriptorSetInterface<'a, R: RendererBackend> {
    const INTERFACE: DescriptorSetDescription<'static>;

    fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>);
}

/// Description of a vertex attribute.
#[derive(Copy, Clone, Debug)]
pub struct TypedVertexInputAttributeDescription<'tcx> {
    pub location: u32,
    pub ty: &'tcx TypeDesc<'tcx>,
    pub format: Format,
    pub offset: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct VertexInputBufferDescription<'tcx> {
    pub elements: &'tcx [TypedVertexInputAttributeDescription<'tcx>],
    pub stride: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct FragmentOutputDescription {
    // nothing yet, we just care about the count
}

///
/// Trait implemented by types that represent vertex data in a vertex buffer.
/// This is used to automatically infer the vertex layout.
///
/// ```rust
/// #[derive(VertexLayout)]
/// #[repr(C)]
/// struct MyVertexType {
///     position: Vec3,
///     normals: Vec3,
///     tangents: Vec3,
///     texcoords: Vec2,
/// }
/// ```
pub trait VertexBuffer {
    const DESCRIPTION: &'static VertexInputBufferDescription<'static>;
}

/// Trait implemented by types that are layout-compatible with an specific
/// to GLSL/SPIR-V type.
/// An implementation is provided for most primitive types and arrays of primitive types.
/// Structs can derive it automatically with `#[derive(BufferLayout)]`
pub trait BufferLayout {
    const TYPE: TypeDesc<'static>;
}

/*

/// An input buffer for vertex data
#[derive(Clone, Debug)]
pub struct VertexBufferDescriptor {
    pub name: Option<String>,
    pub index: u32,
    pub layout: &'static VertexLayout,
}*/

/// An input buffer for indices
#[derive(Clone, Debug)]
pub struct IndexBufferDescriptor {
    pub format: Format,
}

/*
/// A trait defined for types that can be bound to the pipeline as an image.
pub trait ImageInterface: Into<TextureAny> + 'static {
    fn get_data_type() -> Option<TextureDataType>;
    fn get_dimensions() -> Option<TextureDimensions>;
}

impl ImageInterface for ImageHandle {
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
*/

/// Trait implemented by types that can serve as a vertex attribute.
pub unsafe trait VertexAttributeType {
    /// The equivalent type descriptor (the type seen by the shader).
    const EQUIVALENT_TYPE: TypeDesc<'static>;
    /// Returns the corresponding data format (the layout of the data in memory).
    const FORMAT: Format;
}

macro_rules! impl_vertex_attrib_type {
    ($t:ty, $equiv:expr, $fmt:ident) => {
        unsafe impl VertexAttributeType for $t {
            const EQUIVALENT_TYPE: TypeDesc<'static> = $equiv;
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
pub unsafe trait IndexElementType {
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

/*
/// Trait implemented by types that can be bound to the pipeline with a
/// variant of glProgramUniform
/// An implementation is provided for most primitive types .
pub unsafe trait UniformInterface {
    fn type_desc() -> &'static TypeDesc;
}

macro_rules! impl_uniform_type {
    ($t:ty, $tydesc:expr) => {
        unsafe impl BufferLayout for $t {
            fn type_desc() -> &'static TypeDesc {
                static DESC: TypeDesc = $tydesc;
                &DESC
            }
        }
        unsafe impl UniformInterface for $t {
            fn type_desc() -> &'static TypeDesc {
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
*/

/*
/// Trait implemented by types that can be bound to the pipeline as a buffer object
pub unsafe trait BufferInterface {
    /// Get the layout of the buffer data, if it is known.
    fn layout() -> Option<&'static TypeDesc>;
}
*/

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

/*
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
}*/

pub trait PipelineInterfaceVisitor<'a, R: RendererBackend> {
    /// `#[descriptor_set]`
    fn visit_descriptor_sets(&mut self, descriptor_sets: &[&'a R::DescriptorSet]);
    /// `#[vertex_input(index)]`
    fn visit_vertex_buffers(&mut self, buffer: &[&'a R::Buffer]);
    /// `#[index_buffer]`
    fn visit_index_buffer(&mut self, buffer: &'a R::Buffer);
    /// `#[fragment_output]`
    fn visit_framebuffer(&mut self, framebuffer: &'a R::Framebuffer);

    /// `#[viewports]`
    fn visit_dynamic_viewports(&mut self, first: u32, viewports: &[Viewport]);
    /// `#[viewport]`
    fn visit_dynamic_viewport_all(&mut self, viewport: &Viewport);
    /// `#[scissors]`
    fn visit_dynamic_scissors(&mut self, first: u32, scissors: &[ScissorRect]);
    /// `#[scissor]`
    fn visit_dynamic_scissor_all(&mut self, scissor: &ScissorRect);

    //fn visit_data(&self, binding: u32, data: &[u8]);
}

/// 'static bound for getting the typeid
pub trait PipelineInterface<'a, R: RendererBackend> {
    const VERTEX_INPUT_INTERFACE: &'static [VertexInputBufferDescription<'static>];
    const FRAGMENT_OUTPUT_INTERFACE: &'static [FragmentOutputDescription];
    const DESCRIPTOR_SET_INTERFACE: &'static [DescriptorSetDescription<'static>];

    fn do_visit(&self, visitor: &mut PipelineInterfaceVisitor<'a, R>);

    // Use this interface when rust supports impl Trait in Traits
    /*fn vertex_inputs<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::Buffer> + 'a;
    fn fragment_outputs<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::Image> + 'a;
    fn descriptor_sets<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::DescriptorSet> + 'a;
    fn index_buffer(&self) -> Option<R::BufferHandle>;*/

    // misc. render states
    // fn viewports<'a>(&'a self) -> Option<impl Iterator<Item=&'a Viewport> + 'a>;
    // fn scissor_rects<'a>(&'a self) -> Option<impl Iterator<Item=&'a ScissorRect> + 'a>;
}
