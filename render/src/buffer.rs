use crate::{
    typedesc::{PrimitiveType, TypeDesc},
    Backend,
};
pub use autograph_render_macros::StructuredBufferData;
use std::{marker::PhantomData, mem};

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
    pub &'a B::Buffer,
    pub(crate) PhantomData<&'a T>,
);

impl<'a, B: Backend, T: BufferData + ?Sized> Buffer<'a, B, T> {
    /*pub fn byte_size(&self) -> u64 {
        traits::Buffer::size(self.0)
    }*/
    pub fn into_typeless(self) -> BufferTypeless<'a, B> {
        BufferTypeless(self.0)
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
    const TYPE: &'static TypeDesc<'static>;
}

macro_rules! impl_structured_type {
    ($t:ty, $tydesc:expr) => {
        unsafe impl StructuredBufferData for $t {
            const TYPE: &'static TypeDesc<'static> = $tydesc;
        }
    };
}

// Boolean type for use in GLSL interfaces
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

impl_structured_type!(BoolU32, &TypeDesc::Primitive(PrimitiveType::UnsignedInt));
impl_structured_type!(f32, &TypeDesc::Primitive(PrimitiveType::Float));
impl_structured_type!([f32; 2], &TypeDesc::Vector(PrimitiveType::Float, 2));
impl_structured_type!([f32; 3], &TypeDesc::Vector(PrimitiveType::Float, 3));
impl_structured_type!([f32; 4], &TypeDesc::Vector(PrimitiveType::Float, 4));
impl_structured_type!(i32, &TypeDesc::Primitive(PrimitiveType::Int));
impl_structured_type!([i32; 2], &TypeDesc::Vector(PrimitiveType::Int, 2));
impl_structured_type!([i32; 3], &TypeDesc::Vector(PrimitiveType::Int, 3));
impl_structured_type!([i32; 4], &TypeDesc::Vector(PrimitiveType::Int, 4));
impl_structured_type!([[f32; 2]; 2], &TypeDesc::Matrix(PrimitiveType::Float, 2, 2));
impl_structured_type!([[f32; 3]; 3], &TypeDesc::Matrix(PrimitiveType::Float, 3, 3)); // TODO: this is wrong! bad size and alignments
impl_structured_type!([[f32; 4]; 4], &TypeDesc::Matrix(PrimitiveType::Float, 4, 4));

// array impls
unsafe impl<T: StructuredBufferData + Copy> StructuredBufferData for [T; 32] {
    // issue: need the stride of the array?
    const TYPE: &'static TypeDesc<'static> = &TypeDesc::Array(T::TYPE, 32, mem::size_of::<T>());
}

#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Vec2,
    &TypeDesc::Vector(PrimitiveType::Float, 2)
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Vec3,
    &TypeDesc::Vector(PrimitiveType::Float, 3)
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Vec4,
    &TypeDesc::Vector(PrimitiveType::Float, 4)
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Mat2,
    &TypeDesc::Matrix(PrimitiveType::Float, 2, 2)
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Mat3,
    &TypeDesc::Matrix(PrimitiveType::Float, 3, 3)
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Mat4,
    &TypeDesc::Matrix(PrimitiveType::Float, 4, 4)
);
#[cfg(feature = "glm")]
impl_structured_type!(
    nalgebra_glm::Mat4x3,
    &TypeDesc::Matrix(PrimitiveType::Float, 4, 3)
);
