use crate::*;

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    Int,
    UnsignedInt,
    Half, //?
    Float,
    Double,
    Bool,
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
    Struct(&'tcx [(usize, &'tcx TypeDesc<'tcx>)]),
    /// An image type.
    Image(ImageDataType, Option<Format>),
    SampledImage(ImageDataType, Option<Format>),
    Void,
    Pointer(&'tcx TypeDesc<'tcx>),
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

//--------------------------------------------------------------------------------------------------

/// Trait implemented by types that are layout-compatible with an specific
/// to GLSL/SPIR-V type.
/// An implementation is provided for most primitive types and arrays of primitive types.
/// Structs can derive it automatically with `#[derive(BufferLayout)]`
pub unsafe trait BufferLayout {
    const TYPE: &'static TypeDesc<'static>;
}

macro_rules! impl_buffer_layout_type {
    ($t:ty, $tydesc:expr) => {
        unsafe impl BufferLayout for $t {
            const TYPE: &'static TypeDesc<'static> = $tydesc;
        }
    };
}

impl_buffer_layout_type!(f32, &TypeDesc::Primitive(PrimitiveType::Float));
impl_buffer_layout_type!([f32; 2], &TypeDesc::Vector(PrimitiveType::Float, 2));
impl_buffer_layout_type!([f32; 3], &TypeDesc::Vector(PrimitiveType::Float, 3));
impl_buffer_layout_type!([f32; 4], &TypeDesc::Vector(PrimitiveType::Float, 4));
impl_buffer_layout_type!(i32, &TypeDesc::Primitive(PrimitiveType::Int));
impl_buffer_layout_type!([i32; 2], &TypeDesc::Vector(PrimitiveType::Int, 2));
impl_buffer_layout_type!([i32; 3], &TypeDesc::Vector(PrimitiveType::Int, 3));
impl_buffer_layout_type!([i32; 4], &TypeDesc::Vector(PrimitiveType::Int, 4));
impl_buffer_layout_type!([[f32; 2]; 2], &TypeDesc::Matrix(PrimitiveType::Float, 2, 2));
impl_buffer_layout_type!([[f32; 3]; 3], &TypeDesc::Matrix(PrimitiveType::Float, 3, 3));
impl_buffer_layout_type!([[f32; 4]; 4], &TypeDesc::Matrix(PrimitiveType::Float, 4, 4));

#[cfg(feature = "glm-types")]
impl_buffer_layout_type!(
    nalgebra_glm::Vec2,
    &TypeDesc::Vector(PrimitiveType::Float, 2)
);
#[cfg(feature = "glm-types")]
impl_buffer_layout_type!(
    nalgebra_glm::Vec3,
    &TypeDesc::Vector(PrimitiveType::Float, 3)
);
#[cfg(feature = "glm-types")]
impl_buffer_layout_type!(
    nalgebra_glm::Vec4,
    &TypeDesc::Vector(PrimitiveType::Float, 4)
);
#[cfg(feature = "glm-types")]
impl_buffer_layout_type!(
    nalgebra_glm::Mat2,
    &TypeDesc::Matrix(PrimitiveType::Float, 2, 2)
);
#[cfg(feature = "glm-types")]
impl_buffer_layout_type!(
    nalgebra_glm::Mat3,
    &TypeDesc::Matrix(PrimitiveType::Float, 3, 3)
);
#[cfg(feature = "glm-types")]
impl_buffer_layout_type!(
    nalgebra_glm::Mat4,
    &TypeDesc::Matrix(PrimitiveType::Float, 4, 4)
);
#[cfg(feature = "glm-types")]
impl_buffer_layout_type!(
    nalgebra_glm::Mat4x3,
    &TypeDesc::Matrix(PrimitiveType::Float, 4, 3)
);

//--------------------------------------------------------------------------------------------------
/*#[derive(Copy, Clone, Debug)]
pub struct DescriptorSetDescription<'tcx> {
    pub descriptors: &'tcx [DescriptorSetLayoutBinding<'tcx>],
}*/

pub trait DescriptorSetInterfaceVisitor<'a, R: RendererBackend> {
    fn visit_buffer(
        &mut self,
        binding: u32,
        buffer: BufferTypeless<'a, R>,
        offset: usize,
        size: usize,
    );
    fn visit_sampled_image(
        &mut self,
        binding: u32,
        image: Image<'a, R>,
        sampler: &SamplerDescription,
    );

    //fn visit_vertex_input<'a>(&self, buffer: &'a R::Buffer);
    //fn visit_fragment_output<'a>(&self, image: &'a R::Image);
    //fn visit_data(&self, binding: u32, data: &[u8]);
}

pub trait DescriptorSetInterface<'a, R: RendererBackend> {
    const INTERFACE: &'static [DescriptorSetLayoutBinding<'static>];
    fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>);
}

pub trait DescriptorInterface<'a, R: RendererBackend> {
    const TYPE: Option<&'static TypeDesc<'static>>;
    fn do_visit(&self, binding_index: u32, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>);
}

impl<'a, R: RendererBackend> DescriptorInterface<'a, R> for BufferTypeless<'a, R> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
    fn do_visit(
        &self,
        binding_index: u32,
        visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>,
    ) {
        visitor.visit_buffer(binding_index, *self, 0, self.byte_size() as usize);
    }
}

impl<'a, R: RendererBackend, T: BufferData + ?Sized + BufferLayout> DescriptorInterface<'a, R>
    for Buffer<'a, R, T>
{
    const TYPE: Option<&'static TypeDesc<'static>> = Some(<T as BufferLayout>::TYPE);
    fn do_visit(
        &self,
        binding_index: u32,
        visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>,
    ) {
        visitor.visit_buffer(binding_index, (*self).into(), 0, self.byte_size() as usize);
    }
}

impl<'a, R: RendererBackend> DescriptorInterface<'a, R> for Image<'a, R> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
    fn do_visit(
        &self,
        binding_index: u32,
        visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>,
    ) {
        visitor.visit_sampled_image(
            binding_index,
            *self,
            &SamplerDescription::LINEAR_MIPMAP_LINEAR,
        );
    }
}

impl<'a, R: RendererBackend> DescriptorInterface<'a, R> for SampledImage<'a, R> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
    fn do_visit(
        &self,
        binding_index: u32,
        visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>,
    ) {
        visitor.visit_sampled_image(binding_index, Image(self.0), &self.1);
    }
}

//--------------------------------------------------------------------------------------------------
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

/// An input buffer for indices
#[derive(Clone, Debug)]
pub struct IndexBufferDescriptor {
    pub format: Format,
}

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

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
pub struct FragmentOutputDescription {
    // nothing yet, we just care about the count
}

//--------------------------------------------------------------------------------------------------
pub trait PipelineInterfaceVisitor<'a, R: RendererBackend> {
    fn visit_descriptor_sets(&mut self, descriptor_sets: &[DescriptorSet<'a, R>]);
    fn visit_vertex_buffers(&mut self, buffer: &[BufferTypeless<'a, R>]);
    fn visit_index_buffer(&mut self, buffer: BufferTypeless<'a, R>, offset: usize, ty: IndexType);
    fn visit_framebuffer(&mut self, framebuffer: Framebuffer<'a, R>);
    fn visit_dynamic_viewports(&mut self, viewports: &[Viewport]);
    fn visit_dynamic_scissors(&mut self, scissors: &[ScissorRect]);
}

pub trait PipelineInterface<'a, R: RendererBackend> {
    const VERTEX_INPUT_INTERFACE: &'static [VertexInputBufferDescription<'static>];
    const FRAGMENT_OUTPUT_INTERFACE: &'static [FragmentOutputDescription];
    const DESCRIPTOR_SET_INTERFACE: &'static [&'static [DescriptorSetLayoutBinding<'static>]];

    fn do_visit(&self, visitor: &mut PipelineInterfaceVisitor<'a, R>);

    // Use this interface when rust supports impl Trait in Traits
    /*fn vertex_inputs<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::Buffer> + 'a;
    fn fragment_outputs<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::Image> + 'a;
    fn descriptor_sets<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::DescriptorSet> + 'a;
    fn index_buffer(&self) -> Option<R::BufferHandle>;*/
}
