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
///
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

impl<'a, R: RendererBackend> From<BufferTypeless<'a,R>> for Descriptor<'a, R> {
    fn from(buffer: BufferTypeless<'a, R>) -> Self {
        let size = buffer.byte_size() as usize;
        Descriptor::Buffer {
            buffer,
            offset: 0,
            size
        }
    }
}

impl<'a, R: RendererBackend, T: BufferData+?Sized> From<Buffer<'a,R,T>> for Descriptor<'a, R> {
    fn from(buffer: Buffer<'a, R, T>) -> Self {
        // TODO pass/check type info?
        buffer.into_typeless().into()
    }
}

impl<'a, R: RendererBackend> From<(Image<'a, R>, &SamplerDescription)> for Descriptor<'a, R> {
    fn from(img_sampler: (Image<'a, R>, &SamplerDescription)) -> Self {
        Descriptor::SampledImage(img_sampler.0.into_sampled(img_sampler.1.clone()))
    }
}

impl<'a, R: RendererBackend> From<SampledImage<'a,R>> for Descriptor<'a, R> {
    fn from(img: SampledImage<'a,R>) -> Self {
        Descriptor::SampledImage(img)
    }
}

/// Visitor acceped by [DescriptorSetInterface].
pub trait DescriptorSetInterfaceVisitor<'a, R: RendererBackend> {
    fn visit_descriptors(&mut self, descriptors: impl IntoIterator<Item=Descriptor<'a,R>>);
}

/// Trait implemented by types that can be converted to descriptor sets.
///
/// This trait can be automatically derived for structs via a custom derive, each field
/// representing either one or an array of descriptor bindings.
/// All fields should implement [DescriptorInterface] : see the documentation
/// of [DescriptorInterface] for implementors available by default.
///
/// ```
/// #[derive(DescriptorSetInterface)]
/// pub struct PerObjectSet<'a, R: RendererBackend> {
///     ...
/// }
/// ```
///
pub trait DescriptorSetInterface<'a, R: RendererBackend> {
    /// List of binding descriptions. This can be used to build a [DescriptorSetLayout].
    const INTERFACE: &'static [DescriptorSetLayoutBinding<'static>];
    /// Passes all bindings in the set to the given visitor.
    fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>);
}

/// Trait implemented by types that can be turned into descriptors.
///
/// This trait is implemented by default for buffer objects, buffer slices, and images.
pub trait DescriptorInterface<'a, R: RendererBackend>: Into<Descriptor<'a, R>>
{
    /// Type information about the content of the data referenced by the descriptor.
    const TYPE: Option<&'static TypeDesc<'static>>;
}

impl<'a, R: RendererBackend> DescriptorInterface<'a, R> for BufferTypeless<'a, R> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
}

// TODO: no impl for T: !BufferLayout, must use specialization
impl<'a, R: RendererBackend, T: BufferData + ?Sized + BufferLayout> DescriptorInterface<'a, R>
    for Buffer<'a, R, T>
{
    // T: BufferLayout so we have type info about the contents
    const TYPE: Option<&'static TypeDesc<'static>> = Some(<T as BufferLayout>::TYPE);
}

impl<'a, R: RendererBackend> DescriptorInterface<'a, R> for SampledImage<'a, R> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
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

/// Describes the layout of vertex data inside a single vertex buffer.
#[derive(Copy, Clone, Debug)]
pub struct VertexInputBufferDescription<'tcx> {
    /// Description of individual vertex attributes inside the buffer.
    pub elements: &'tcx [TypedVertexInputAttributeDescription<'tcx>],
    /// Number of bytes to go to the next element.
    pub stride: usize,
}

/// Descriptor for a vertex buffer.
#[derive(Copy, Clone, Debug)]
pub struct VertexBufferDescriptor<'a, 'tcx, R: RendererBackend>
{
    /// Buffer containing vertex data.
    pub buffer: BufferTypeless<'a, R>,
    /// Layout of vertex data.
    pub desc: &'tcx VertexInputBufferDescription<'tcx>,
    /// Offset to the start of vertex data in the buffer.
    pub offset: u64,
}

/// Trait implemented by types that represent vertex data in a vertex buffer.
///
/// This is used to automatically infer the vertex layout.
///
/// TODO explain unsafety.
///
/// It can be automatically derived from repr(C) structs, provided that all the fields implement
/// [VertexAttributeType] :
///
/// ```rust
/// #[derive(VertexData)]
/// #[repr(C)]
/// struct MyVertexType {
///     position: Vec3,
///     normals: Vec3,
///     tangents: Vec3,
///     texcoords: Vec2,
/// }
/// ```
pub unsafe trait VertexData: BufferData {
    const DESCRIPTION: &'static VertexInputBufferDescription<'static>;
}

/// Descriptor for an index buffer.
#[derive(Clone, Debug)]
pub struct IndexBufferDescriptor<'a, R: RendererBackend> {
    /// Buffer containing index data.
    pub buffer: BufferTypeless<'a, R>,
    /// Format of indices.
    pub format: IndexFormat,
    /// Offset to the start of index data in the buffer.
    pub offset: u64,
}

/// Trait implemented by types that can serve as indices.
pub unsafe trait IndexData: BufferData {
    /// Index type.
    const FORMAT: IndexFormat;
}

// typed buffer -> vertex buffer descriptor
impl<'a, 'tcx, T, R: RendererBackend> From<Buffer<'a, R, [T]>> for VertexBufferDescriptor<'a, 'tcx, R> where T: VertexData {
    fn from(buf: Buffer<'a, R, [T]>) -> Self {
        VertexBufferDescriptor {
            offset: 0,
            buffer: buf.into(),
            desc: <T as VertexData>::DESCRIPTION,
        }
    }
}

// typed buffer -> index buffer descriptor
impl<'a, T, R: RendererBackend> From<Buffer<'a, R, [T]>> for IndexBufferDescriptor<'a, R> where T: IndexData {
    fn from(buf: Buffer<'a, R, [T]>) -> Self {
        IndexBufferDescriptor {
            offset: 0,
            buffer: buf.into(),
            format: <T as IndexData>::FORMAT,
        }
    }
}


/// Trait implemented by types that can serve as a vertex attribute.
pub unsafe trait VertexAttributeType {
    /// The equivalent type descriptor (the type seen by the shader).
    const EQUIVALENT_TYPE: TypeDesc<'static>;
    /// Returns the corresponding data format (the layout of the data in memory).
    const FORMAT: Format;
}

// Vertex attribute types --------------------------------------------------------------------------
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

// Index data types --------------------------------------------------------------------------------
macro_rules! impl_index_data {
    ($t:ty, $fmt:ident) => {
        unsafe impl IndexData for $t {
            const FORMAT: IndexFormat = IndexFormat::$fmt;
        }
    };
}

impl_index_data!(u16, U16);
impl_index_data!(u32, U32);

// Vertex data types -------------------------------------------------------------------------------


//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
pub struct FragmentOutputDescription {
    // nothing yet, we just care about the count
}


//--------------------------------------------------------------------------------------------------
pub trait PipelineInterfaceVisitor<'a, R: RendererBackend> {
    fn visit_descriptor_sets<I: IntoIterator<Item=DescriptorSet<'a,R>>>(&mut self, descriptor_sets: I);
    fn visit_vertex_buffers<'tcx, I: IntoIterator<Item=VertexBufferDescriptor<'a,'tcx,R>>>(&mut self, buffers: I);
    fn visit_index_buffer(&mut self, buffer: IndexBufferDescriptor<'a, R>);
    fn visit_framebuffer(&mut self, framebuffer: Framebuffer<'a, R>);
    fn visit_dynamic_viewports<I: IntoIterator<Item=Viewport>>(&mut self, viewports: I);
    fn visit_dynamic_scissors<I: IntoIterator<Item=ScissorRect>>(&mut self, scissors: I);
}


///
/// Describes pipeline states to set before issuing a draw or compute call.
///
/// #### Custom derive
/// It is possible to automatically derive [PipelineInterface] for structs with named fields.
///
/// By default, the implementation is generic over the backend type, and expects the struct to have
/// one lifetime parameter (`'a`) and a single type parameter for the renderer backend
/// (`T: RendererBackend`).
///
/// Attributes define the interpretation of struct fields in terms of pipeline bindings.
/// The field must be convertible via [Into] to the target binding type:
/// * `descriptor_set(index=n)` specifies a descriptor set.
/// The field must implement `Into<DescriptorSet>`.
/// * `vertex_buffer` specifies a vertex buffer. The field must implement `Into<BufferTypeless>`.
/// * `index_buffer` specifies an index buffer. The field must implement `Into<BufferTypeless>`.
///   Annotating more than one field with this attribute is an error.
/// * `viewport` specifies a viewport. The field must implement `Into<Viewport>`.
/// * `scissor` specifies a scissor rectangle. The field must implement `Into<ScissorRect>`.
///
/// It is also possible to specify arrays of pipeline bindings:
/// * `descriptor_set_array(base_index=n)` specifies an array of descriptor sets.
///   The field must implement `IntoIterator<DescriptorSet>`.
/// * `vertex_buffer_array(base_index=n)` specifies an array of vertex buffers.
///   The field must implement `IntoIterator<BufferTypeless>`.
/// * `viewport_array(base_index=n)` specifies an array of viewports.
///   The field must implement `IntoIterator<Viewport>`.
/// * `scissor_array(base_index=n)` specifies an array of scissor rectangles.
///   The field must implement `IntoIterator<ScissorRect>`.
///
/// When a binding index is required, it is derived from the order of appearance of the binding in
/// the struct. For instance:
///
/// ```
/// #[derive(PipelineInterface)]
/// pub struct ExampleInterface<'a, R: RendererBackend> {
///     #[descriptor_set] ds_a: DescriptorSet<'a, R>,     // index 0
///     ...
///     #[descriptor_set] ds_b: DescriptorSet<'a, R>,     // index 1
///     ...
///     #[descriptor_set_array] dss: DescriptorSet<'a, R>,   // indices 2..2+dss.len
///     ...
///     #[descriptor_set] ds_c: DescriptorSet<'a, R>,   // index dss.len
/// }
/// ```
///
/// #### Example
///
///```
/// #[derive(PipelineInterface)]
/// pub struct ExampleInterface<'a, R: Render> {
///    #[framebuffer]
///    pub framebuffer: Framebuffer<'a>,
///    #[descriptor_set]
///    pub per_object: DescriptorSet<'a>,   // DS index 0
///    #[viewport]
///    pub viewport: Viewport,
///    // Buffer<T> implements Into<BufferTypeless>
///    #[vertex_buffer]
///    pub vertex_buffer: Buffer<'a, [Vertex]>,  // VB index 0
///    #[descriptor_set]
///    pub other: DescriptorSet<'a>,  // DS index 1
/// }
/// ```
///
pub trait PipelineInterface<'a, R: RendererBackend> {
    const VERTEX_INPUT_INTERFACE: &'static [VertexInputBufferDescription<'static>];
    const FRAGMENT_OUTPUT_INTERFACE: &'static [FragmentOutputDescription];
    const DESCRIPTOR_SET_INTERFACE: &'static [&'static [DescriptorSetLayoutBinding<'static>]];

    fn do_visit<V: PipelineInterfaceVisitor<'a,R>>(&self, visitor: &mut V);

    // Use this interface when rust supports impl Trait in Traits
    /*fn vertex_inputs<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::Buffer> + 'a;
    fn fragment_outputs<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::Image> + 'a;
    fn descriptor_sets<'a,'rcx>(&'a self) -> impl Iterator<Item=&'rcx R::DescriptorSet> + 'a;
    fn index_buffer(&self) -> Option<R::BufferHandle>;*/
}
