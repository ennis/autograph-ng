//! Descriptors
use crate::buffer::Buffer;
use crate::buffer::BufferData;
use crate::buffer::BufferTypeless;
use crate::buffer::StructuredBufferData;
use crate::image::SampledImage;
use crate::image::SamplerDescription;
use crate::pipeline::ShaderStageFlags;
use crate::traits;
use crate::typedesc::TypeDesc;
use crate::Arena;
pub use autograph_render_macros::DescriptorSetInterface;
use std::any::TypeId;
use std::marker::PhantomData;

/// Descriptor set layout.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct DescriptorSetLayout<'a>(pub &'a dyn traits::DescriptorSetLayout);

/// Descriptor set.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct DescriptorSetTypeless<'a>(pub &'a dyn traits::DescriptorSet);

/// Descriptor set.
#[derive(Debug)]
#[repr(transparent)]
pub struct DescriptorSet<'a, T: DescriptorSetInterface<'a>>(
    pub &'a dyn traits::DescriptorSet,
    pub(crate) PhantomData<T>,
);

impl<'a, T: DescriptorSetInterface<'a>> Clone for DescriptorSet<'a, T> {
    fn clone(&self) -> Self {
        DescriptorSet(self.0, PhantomData)
    }
}

impl<'a, T: DescriptorSetInterface<'a>> Copy for DescriptorSet<'a, T> {}

/// Represents an entry (binding) in a descriptor set layout.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DescriptorSetLayoutBinding<'tcx> {
    /// Binding index
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Which shader stages will see this descriptor
    pub stage_flags: ShaderStageFlags,
    /// TODO How many descriptors in the binding? Should be 1
    pub count: usize,
    /// Precise description of the expected data type (image format, layout of buffer data, etc.).
    ///
    /// Can be None if no type information is available for this binding.
    pub tydesc: Option<&'tcx TypeDesc<'tcx>>,
}

/// Descriptor type
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum DescriptorType {
    Sampler, // TODO
    SampledImage,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
    InputAttachment,
}

/// A reference to a resource used by one or more shader stages in the pipeline.
#[derive(Copy, Clone)]
pub enum Descriptor<'a> {
    SampledImage {
        img: &'a dyn traits::Image,
        sampler: SamplerDescription,
    },
    Image {
        img: Image<'a, R>,
    },
    Buffer {
        buffer: BufferTypeless<'a, R>,
        offset: usize,
        size: usize,
    },
    // TODO push constants
    Empty,
}

/// Visitor acceped by [DescriptorSetInterface].
pub trait DescriptorSetInterfaceVisitor<'a> {
    fn visit_descriptors(&mut self, descriptors: impl IntoIterator<Item = Descriptor<'a>>);
}

/// Layout of a descriptor set.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DescriptorSetLayoutDescription<'tcx> {
    pub bindings: &'tcx [DescriptorSetLayoutBinding<'tcx>],
    pub typeid: Option<TypeId>,
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
/// pub struct PerObjectSet<'a> {
///     ...
/// }
/// ```
///
pub trait DescriptorSetInterface<'a> {
    /// List of binding descriptions. This can be used to build a [DescriptorSetLayout].
    const LAYOUT: DescriptorSetLayoutDescription<'static>;

    /// A 'static marker type that uniquely identifies Self: this is for getting a TypeId.
    type UniqueType: 'static;
    type IntoInterface: DescriptorSetInterface<'a>;

    /// Converts this object into a DescriptorSet.
    ///
    /// If necessary, creates a new descriptor set.
    fn into_descriptor_set(self, arena: &'a Arena) -> DescriptorSet<'a, Self::IntoInterface>
    where
        Self: Sized;

    // Passes all bindings in the set to the given visitor.
    //fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a>);
}

/// Trait implemented by types that can be turned into descriptors.
///
/// This trait is implemented by default for buffer objects, buffer slices, and images.
pub trait DescriptorInterface<'a>: Into<Descriptor<'a>> {
    /// Type information about the content of the data referenced by the descriptor.
    const TYPE: Option<&'static TypeDesc<'static>>;
}

impl<'a> DescriptorInterface<'a> for BufferTypeless<'a> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
}

// TODO: no impl for T: !BufferLayout, must use specialization
impl<'a, T: BufferData + ?Sized + StructuredBufferData> DescriptorInterface<'a> for Buffer<'a, T> {
    // T: BufferLayout so we have type info about the contents
    const TYPE: Option<&'static TypeDesc<'static>> = Some(<T as StructuredBufferData>::TYPE);
}

impl<'a> DescriptorInterface<'a> for SampledImage<'a> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
}

impl<'a> From<BufferTypeless<'a>> for Descriptor<'a> {
    fn from(buffer: BufferTypeless<'a>) -> Self {
        let size = buffer.byte_size() as usize;
        Descriptor::Buffer {
            buffer: buffer.0,
            offset: 0,
            size,
        }
    }
}

impl<'a, T: BufferData + ?Sized> From<Buffer<'a, T>> for Descriptor<'a> {
    fn from(buffer: Buffer<'a, T>) -> Self {
        // TODO pass/check type info?
        buffer.into_typeless().into()
    }
}

/*
impl<'a> From<(Image<'a>, &SamplerDescription)> for Descriptor<'a> {
    fn from(img_sampler: (Image<'a>, &SamplerDescription)) -> Self {
        Descriptor::SampledImage(img_sampler.0.into_sampled(img_sampler.1.clone()))
    }
}*/

impl<'a> From<SampledImage<'a>> for Descriptor<'a> {
    fn from(img: SampledImage<'a>) -> Self {
        Descriptor::SampledImage {
            img: img.0,
            sampler: img.1,
        }
    }
}

impl<'a, T: DescriptorSetInterface<'a>> DescriptorSetInterface<'a> for DescriptorSet<'a, T> {
    const LAYOUT: DescriptorSetLayoutDescription<'static> =
        <T as DescriptorSetInterface<'a>>::LAYOUT;

    type UniqueType = <T as DescriptorSetInterface<'a>>::UniqueType;
    type IntoInterface = T;

    fn into_descriptor_set(self, _: &'a Arena) -> DescriptorSet<'a, T> {
        self
    }
}

#[cfg(feature = "intellij")]
impl<'a, R: RendererBackend> Copy for Descriptor<'a,R> {}
*/
