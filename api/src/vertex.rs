use crate::{
    buffer::{Buffer, BufferData},
    format::Format,
    typedesc::{PrimitiveType, TypeDesc},
};

use crate::Backend;
pub use autograph_api_macros::VertexData;

/// Describes the type of indices contained in an index buffer.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum IndexFormat {
    /// 16-bit unsigned integer indices
    U16,
    /// 32-bit unsigned integer indices
    U32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Semantic<'tcx> {
    pub name: &'tcx str,
    /// Index relative to the base semantic index of the VertexInputBinding
    pub index: u32,
}

/// Description of a vertex attribute within a vertex layout.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VertexLayoutElement<'tcx> {
    pub semantic: Option<Semantic<'tcx>>,
    pub format: Format,
    pub offset: u32,
}

/// Describes the layout of vertex data inside a single vertex buffer.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VertexLayout<'tcx> {
    /// Description of individual vertex attributes inside the buffer.
    pub elements: &'tcx [VertexLayoutElement<'tcx>],
    /// Number of bytes to go to the next element.
    pub stride: usize,
}

/// Descriptor for a vertex buffer.
/// TODO support host references.
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct VertexBufferView<'a, B: Backend> {
    /// Buffer containing vertex data.
    pub(crate) buffer: &'a B::Buffer,
    /// Layout of vertex data.
    pub(crate) stride: usize,
    /// Offset to the start of vertex data in the buffer.
    pub(crate) offset: usize,
}

impl<'a, B: Backend> VertexBufferView<'a, B> {
    pub fn buffer(&self) -> &'a B::Buffer {
        self.buffer
    }
    pub fn stride(&self) -> usize {
        self.stride
    }
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl<'a, B: Backend, V: VertexData> From<Buffer<'a, B, [V]>> for VertexBufferView<'a, B> {
    fn from(buf: Buffer<'a, B, [V]>) -> Self {
        VertexBufferView {
            offset: 0,
            buffer: buf.0,
            stride: V::LAYOUT.stride,
        }
    }
}

/*
#[derive(Copy, Clone, Debug)]
pub struct TypedVertexBufferView<'a, B: Backend, V: VertexData> {
    /// Buffer containing vertex data.
    pub(crate) buffer: &'a B::Buffer,
    /// Offset to the start of vertex data in the buffer.
    pub(crate) offset: u64,
    pub(crate) _phantom: PhantomData<&'a V>
}

impl<'a, B: Backend, V: VertexData> From<Buffer<'a, B, [V]>> for TypedVertexBufferView<'a, B, V> {
    fn from(buf: Buffer<'a, B, [V]>) -> Self {
        TypedVertexBufferView {
            offset: 0,
            buffer: buf.0,
            _phantom: PhantomData
        }
    }
}*/

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
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct IndexBufferView<'a, B: Backend> {
    /// Buffer containing index data.
    pub buffer: &'a B::Buffer,
    /// Format of indices.
    pub format: IndexFormat,
    /// Offset to the start of index data in the buffer.
    pub offset: usize,
}

/// Trait implemented by types that can serve as indices.
pub unsafe trait IndexData: BufferData {
    /// Index type.
    const FORMAT: IndexFormat;
}

impl<'a, B: Backend, I: IndexData> From<Buffer<'a, B, [I]>> for IndexBufferView<'a, B> {
    fn from(buf: Buffer<'a, B, [I]>) -> Self {
        IndexBufferView {
            offset: 0,
            buffer: buf.0,
            format: <I as IndexData>::FORMAT,
        }
    }
}

/*
#[derive(Copy, Clone, Debug)]
pub struct TypedIndexBufferView<'a, B: Backend, I: IndexData> {
    pub(crate) buffer: &'a B::Buffer,
    pub(crate) offset: u64,
    pub(crate) _phantom: PhantomData<&'a I>
}

impl<'a, B: Backend, I: IndexData> From<Buffer<'a, B, [I]>> for TypedIndexBufferView<'a, B, I> {
    fn from(buf: Buffer<'a, B, [I]>) -> Self {
        TypedIndexBufferView {
            offset: 0,
            buffer: buf.0,
            _phantom: PhantomData
        }
    }
}*/

//--------------------------------------------------------------------------------------------------
pub trait VertexBufferInterface<'a, B: Backend>: Into<VertexBufferView<'a, B>> {
    type Vertex: VertexData;
}

pub trait IndexBufferInterface<'a, B: Backend>: Into<IndexBufferView<'a, B>> {
    type Index: IndexData;
}

/*
// typed buffer -> vertex buffer descriptor
impl<'a, B: Backend, V> From<TypedVertexBufferView<'a, B, V>> for VertexBufferView<'a, B>
where
    V: VertexData,
{
    fn from(v: TypedVertexBufferView<'a, B, V>) -> Self {
        VertexBufferView {
            offset: v.offset,
            buffer: v.buffer,
            stride: <V as VertexData>::LAYOUT.stride,
        }
    }
}

// typed buffer -> index buffer descriptor
impl<'a, B: Backend, I: IndexData> From<TypedIndexBufferView<'a, B, I>> for IndexBufferView<'a, B> {
    fn from(i: TypedIndexBufferView<'a, B, I>) -> Self {
        IndexBufferView {
            offset: i.offset,
            buffer: i.buffer,
            format: <I as IndexData>::FORMAT,
        }
    }
}*/

impl<'a, 'tcx, B: Backend, V: VertexData> VertexBufferInterface<'a, B> for Buffer<'a, B, [V]> {
    type Vertex = V;
}

impl<'a, B: Backend, I: IndexData> IndexBufferInterface<'a, B> for Buffer<'a, B, [I]> {
    type Index = I;
}

/// Trait implemented by types that can serve as a vertex attribute.
pub unsafe trait VertexAttributeType {
    /// The equivalent type descriptor (the type seen by the shader).
    const EQUIVALENT_TYPE: TypeDesc<'static>;
    /// Returns the corresponding data format (the layout of the data in memory).
    const FORMAT: Format;
}

// Vertex attribute types --------------------------------------------------------------------------
macro_rules! impl_attrib_type {
    ($t:ty, $equiv:expr, $fmt:ident) => {
        unsafe impl VertexAttributeType for $t {
            const EQUIVALENT_TYPE: TypeDesc<'static> = $equiv;
            const FORMAT: Format = Format::$fmt;
        }
    };
}

macro_rules! impl_attrib_prim_type {
    ($t:ty, $prim:ident, $fmt:ident) => {
        unsafe impl VertexAttributeType for $t {
            const EQUIVALENT_TYPE: TypeDesc<'static> = TypeDesc::Primitive(PrimitiveType::$prim);
            const FORMAT: Format = Format::$fmt;
        }
    };
}

macro_rules! impl_attrib_array_type {
    ([$t:ty; $len:expr], $prim:ident, $fmt:ident) => {
        unsafe impl VertexAttributeType for [$t; $len] {
            const EQUIVALENT_TYPE: TypeDesc<'static> = TypeDesc::Vector {
                elem_ty: PrimitiveType::$prim,
                len: $len,
            };
            const FORMAT: Format = Format::$fmt;
        }
    };
}

// F32
impl_attrib_prim_type!(f32, Float, R32_SFLOAT);
impl_attrib_array_type!([f32; 2], Float, R32G32_SFLOAT);
impl_attrib_array_type!([f32; 3], Float, R32G32B32_SFLOAT);
impl_attrib_array_type!([f32; 4], Float, R32G32B32A32_SFLOAT);

// U32
impl_attrib_prim_type!(u32, UnsignedInt, R32_UINT);
impl_attrib_array_type!([u32; 2], UnsignedInt, R32G32_UINT);
impl_attrib_array_type!([u32; 3], UnsignedInt, R32G32B32_UINT);
impl_attrib_array_type!([u32; 4], UnsignedInt, R32G32B32A32_UINT);

impl_attrib_prim_type!(i32, Int, R32_SINT);
impl_attrib_array_type!([i32; 2], Int, R32G32_SINT);
impl_attrib_array_type!([i32; 3], Int, R32G32B32_SINT);
impl_attrib_array_type!([i32; 4], Int, R32G32B32A32_SINT);

// U8
impl_attrib_prim_type!(u16, UnsignedInt, R16_UINT);
impl_attrib_array_type!([u16; 2], UnsignedInt, R16G16_UINT);
impl_attrib_array_type!([u16; 3], UnsignedInt, R16G16B16_UINT);
impl_attrib_array_type!([u16; 4], UnsignedInt, R16G16B16A16_UINT);

impl_attrib_prim_type!(i16, Int, R16_SINT);
impl_attrib_array_type!([i16; 2], Int, R16G16_SINT);
impl_attrib_array_type!([i16; 3], Int, R16G16B16_SINT);
impl_attrib_array_type!([i16; 4], Int, R16G16B16A16_SINT);

// U8
impl_attrib_prim_type!(u8, UnsignedInt, R8_UINT);
impl_attrib_array_type!([u8; 2], UnsignedInt, R8G8_UINT);
impl_attrib_array_type!([u8; 3], UnsignedInt, R8G8B8_UINT);
impl_attrib_array_type!([u8; 4], UnsignedInt, R8G8B8A8_UINT);

impl_attrib_prim_type!(i8, Int, R8_SINT);
impl_attrib_array_type!([i8; 2], Int, R8G8_SINT);
impl_attrib_array_type!([i8; 3], Int, R8G8B8_SINT);
impl_attrib_array_type!([i8; 4], Int, R8G8B8A8_SINT);

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

#[cfg(feature = "glm")]
impl_attrib_type!(
    nalgebra_glm::Vec2,
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 2
    },
    R32G32_SFLOAT
);
#[cfg(feature = "glm")]
impl_attrib_type!(
    nalgebra_glm::Vec3,
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 3
    },
    R32G32B32_SFLOAT
);
#[cfg(feature = "glm")]
impl_attrib_type!(
    nalgebra_glm::Vec4,
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 4
    },
    R32G32B32A32_SFLOAT
);

#[cfg(feature = "glm")]
impl_attrib_type!(
    nalgebra_glm::U8Vec4,
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 4
    },
    R8G8B8A8_UNORM // FIXME why UNORM and not UINT?
);
