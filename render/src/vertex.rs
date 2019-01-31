use crate::buffer::Buffer;
use crate::buffer::BufferData;
use crate::buffer::BufferTypeless;
use crate::format::Format;
use crate::typedesc::PrimitiveType;
use crate::typedesc::TypeDesc;

pub use autograph_render_macros::VertexData;

/// Describes the type of indices contained in an index buffer.
#[derive(Copy, Clone, Debug)]
pub enum IndexFormat {
    /// 16-bit unsigned integer indices
    U16,
    /// 32-bit unsigned integer indices
    U32,
}

/// Description of a vertex attribute within a vertex layout.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TypedVertexInputAttributeDescription<'tcx> {
    pub ty: &'tcx TypeDesc<'tcx>,
    pub format: Format,
    pub offset: u32,
}

/// Describes the layout of vertex data inside a single vertex buffer.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VertexLayout<'tcx> {
    /// Description of individual vertex attributes inside the buffer.
    pub elements: &'tcx [TypedVertexInputAttributeDescription<'tcx>],
    /// Number of bytes to go to the next element.
    pub stride: usize,
}

/// Descriptor for a vertex buffer.
/// TODO support host references.
#[derive(Copy, Clone, Debug)]
pub struct VertexBufferDescriptor<'a, 'tcx> {
    /// Buffer containing vertex data.
    pub buffer: BufferTypeless<'a>,
    /// Layout of vertex data.
    pub layout: &'tcx VertexLayout<'tcx>,
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
    const LAYOUT: VertexLayout<'static>;
}

/// Descriptor for an index buffer.
#[derive(Clone, Debug)]
pub struct IndexBufferDescriptor<'a> {
    /// Buffer containing index data.
    pub buffer: BufferTypeless<'a>,
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

pub trait VertexBufferInterface<'a, 'tcx>: Into<VertexBufferDescriptor<'a, 'tcx>> {
    type Vertex: VertexData;
}

pub trait IndexBufferInterface<'a>: Into<IndexBufferDescriptor<'a>> {
    type Index: IndexData;
}

// typed buffer -> vertex buffer descriptor
impl<'a, 'tcx, T> From<Buffer<'a, [T]>> for VertexBufferDescriptor<'a, 'tcx>
where
    T: VertexData,
{
    fn from(buf: Buffer<'a, [T]>) -> Self {
        VertexBufferDescriptor {
            offset: 0,
            buffer: buf.into(),
            layout: &<T as VertexData>::LAYOUT,
        }
    }
}

// typed buffer -> index buffer descriptor
impl<'a, T> From<Buffer<'a, [T]>> for IndexBufferDescriptor<'a>
where
    T: IndexData,
{
    fn from(buf: Buffer<'a, [T]>) -> Self {
        IndexBufferDescriptor {
            offset: 0,
            buffer: buf.into(),
            format: <T as IndexData>::FORMAT,
        }
    }
}

impl<'a, 'tcx, T> VertexBufferInterface<'a, 'tcx> for Buffer<'a, [T]>
where
    T: VertexData,
{
    type Vertex = T;
}

impl<'a, T> IndexBufferInterface<'a> for Buffer<'a, [T]>
where
    T: IndexData,
{
    type Index = T;
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
impl_vertex_attrib_type!(i32, TypeDesc::Primitive(PrimitiveType::Int), R32_SINT);
impl_vertex_attrib_type!(
    [i32; 2],
    TypeDesc::Vector(PrimitiveType::Int, 2),
    R32G32_SINT
);
impl_vertex_attrib_type!(
    [i32; 3],
    TypeDesc::Vector(PrimitiveType::Int, 3),
    R32G32B32_SINT
);
impl_vertex_attrib_type!(
    [i32; 4],
    TypeDesc::Vector(PrimitiveType::Int, 4),
    R32G32B32A32_SINT
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

#[cfg(feature = "glm-types")]
impl_vertex_attrib_type!(
    nalgebra_glm::Vec2,
    TypeDesc::Vector(PrimitiveType::Float, 2),
    R32G32_SFLOAT
);
#[cfg(feature = "glm-types")]
impl_vertex_attrib_type!(
    nalgebra_glm::Vec3,
    TypeDesc::Vector(PrimitiveType::Float, 3),
    R32G32B32_SFLOAT
);
#[cfg(feature = "glm-types")]
impl_vertex_attrib_type!(
    nalgebra_glm::Vec4,
    TypeDesc::Vector(PrimitiveType::Float, 4),
    R32G32B32A32_SFLOAT
);
