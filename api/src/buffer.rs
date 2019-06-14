use crate::{
    descriptor::{Descriptor, ResourceBindingType, ResourceInterface},
    typedesc::{Layout, PrimitiveType, TypeDesc},
    Backend,
};
pub use autograph_api_macros::StructuredBufferData;
use std::marker::PhantomData;

//--------------------------------------------------------------------------------------------------

/// Marker trait for data that can be uploaded to a GPU buffer
pub trait BufferData: 'static {
    type Element;
    fn len(&self) -> usize;
}

impl<T: Copy + 'static> BufferData for T {
    type Element = T;
    fn len(&self) -> usize {
        1
    }
}

impl<U: BufferData> BufferData for [U] {
    type Element = U;
    fn len(&self) -> usize {
        (&self as &[U]).len()
    }
}

//--------------------------------------------------------------------------------------------------

/// Buffer.
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
#[repr(transparent)]
pub struct Buffer<'a, B: Backend, T: BufferData + ?Sized>(
    pub(crate) &'a B::Buffer,
    pub(crate) PhantomData<&'a T>,
);

impl<'a, B: Backend, T: BufferData + ?Sized> Buffer<'a, B, T> {
    /*pub fn byte_size(&self) -> u64 {
        traits::Buffer::size(self.0)
    }*/
    pub fn inner(&self) -> &'a B::Buffer {
        self.0
    }

    pub fn into_typeless(self) -> BufferTypeless<'a, B> {
        BufferTypeless(self.0)
    }

    pub unsafe fn from_raw(raw: &'a B::Buffer) -> Buffer<'a, B, T> {
        Buffer(raw, PhantomData)
    }
}

/// Buffer without type information.
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
#[repr(transparent)]
pub struct BufferTypeless<'a, B: Backend>(pub &'a B::Buffer);

impl<'a, B: Backend, T: BufferData + ?Sized> From<Buffer<'a, B, T>> for BufferTypeless<'a, B> {
    fn from(from: Buffer<'a, B, T>) -> Self {
        from.into_typeless()
    }
}

/// Buffer slice.
pub struct BufferSlice<'a, B: Backend> {
    pub buffer: BufferTypeless<'a, B>,
    pub offset: usize,
    pub size: usize,
}

//--------------------------------------------------------------------------------------------------

/// Trait implemented by types that are layout-compatible with an specific
/// to GLSL/SPIR-V type.
///
/// An implementation is provided for most primitive types and arrays of primitive types.
/// Structs can derive it automatically with `#[derive(StructuredBufferData)]`
///
/// Unresolved issue: a struct may have alignment requirements
pub unsafe trait StructuredBufferData: BufferData {
    const TYPE: TypeDesc<'static>;
    const LAYOUT: Layout<'static>;
}

macro_rules! impl_structured_type {
    ($t:ty, $tydesc:expr) => {
        unsafe impl StructuredBufferData for $t {
            const TYPE: TypeDesc<'static> = $tydesc;
            const LAYOUT: Layout<'static> =
                Layout::with_size_align(std::mem::size_of::<$t>(), std::mem::align_of::<$t>());
        }
    };
}

// 32-bit-sized boolean type for use in shader interfaces
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BoolU32 {
    False = 0,
    True = 1,
}

impl Default for BoolU32 {
    fn default() -> Self {
        BoolU32::False
    }
}

impl_structured_type!(BoolU32, TypeDesc::Primitive(PrimitiveType::UnsignedInt));
impl_structured_type!(f32, TypeDesc::Primitive(PrimitiveType::Float));
impl_structured_type!(
    [f32; 2],
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 2
    }
);
impl_structured_type!(
    [f32; 3],
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 3
    }
);
impl_structured_type!(
    [f32; 4],
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 4
    }
);
impl_structured_type!(i32, TypeDesc::Primitive(PrimitiveType::Int));
impl_structured_type!(
    [i32; 2],
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Int,
        len: 2
    }
);
impl_structured_type!(
    [i32; 3],
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Int,
        len: 3
    }
);
impl_structured_type!(
    [i32; 4],
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Int,
        len: 4
    }
);
impl_structured_type!(
    [[f32; 2]; 2],
    TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 2,
        columns: 2
    }
);
impl_structured_type!(
    [[f32; 3]; 3],
    TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 3,
        columns: 3
    }
);
impl_structured_type!(
    [[f32; 4]; 4],
    TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 4,
        columns: 4
    }
);

/*
// array impls
unsafe impl<T: StructuredBufferData + Copy> StructuredBufferData for [T; 32] {
    // issue: need the stride of the array?
    const TYPE: &'static TypeDesc<'static> = &TypeDesc::Array {
        T
    }::TYPE, 32, mem::size_of::<T>());
}*/

#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Vec2,
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 2
    }
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Vec3,
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 3
    }
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Vec4,
    TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 4
    }
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Mat2,
    TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 2,
        columns: 2
    }
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Mat3,
    TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 3,
        columns: 3
    }
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Mat4,
    TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 4,
        columns: 4
    }
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Mat4x3,
    TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 4,
        columns: 3
    }
);

//--------------------------------------------------------------------------------------------------
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct TypedConstantBufferView<'a, B: Backend, T: StructuredBufferData> {
    pub(crate) buffer: &'a B::Buffer,
    pub(crate) offset: usize,
    pub(crate) size: Option<usize>,
    pub(crate) _phantom: PhantomData<&'a T>,
}

impl<'a, B: Backend, T: StructuredBufferData> ResourceInterface<'a, B>
    for TypedConstantBufferView<'a, B, T>
{
    const TYPE: ResourceBindingType = ResourceBindingType::ConstantBuffer;
    const DATA_TYPE: Option<&'static TypeDesc<'static>> = Some(&T::TYPE);
    const DATA_LAYOUT: Option<&'static Layout<'static>> = Some(&T::LAYOUT);
    fn into_descriptor(self) -> Descriptor<'a, B> {
        Descriptor::ConstantBuffer {
            buffer: self.buffer,
            offset: self.offset,
            size: self.size,
        }
    }
}

impl<'a, B: Backend, T: StructuredBufferData> From<Buffer<'a, B, T>>
    for TypedConstantBufferView<'a, B, T>
{
    fn from(buf: Buffer<'a, B, T>) -> Self {
        TypedConstantBufferView {
            buffer: buf.0,
            offset: 0,
            size: None,
            _phantom: PhantomData,
        }
    }
}
