//! Descriptors
use crate::{
    buffer::{Buffer, BufferData, BufferTypeless, StructuredBufferData},
    image::{ TextureImageView, SamplerDescription},
    pipeline::ShaderStageFlags,
    typedesc::TypeDesc,
    Backend,
};
use std::marker::PhantomData;
use crate::image::ImageView;

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct HostReference<'a, B: Backend, T: BufferData>(
    pub &'a B::HostReference,
    pub(crate) PhantomData<&'a T>,
);

/// Represents an entry (binding) in a descriptor set layout.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DescriptorBinding<'tcx> {
    /// Binding index
    pub binding: usize,
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
pub enum Descriptor<'a, B: Backend> {
    SampledImage {
        img: &'a B::Image,
        sampler: SamplerDescription,
    },
    Image {
        img: &'a B::Image,
    },
    Buffer {
        buffer: &'a B::Buffer,
        offset: usize,
        size: Option<usize>,
    },
    //HostReference(HostReference<'a, B>),
    Empty,
}

/// Trait implemented by types that can be turned into descriptors.
///
/// This trait is implemented by default for buffer objects, buffer slices, and images.
pub trait DescriptorInterface<'a, B: Backend>: Into<Descriptor<'a, B>> {
    /// Type information about the content of the data referenced by the descriptor.
    const TYPE: Option<&'static TypeDesc<'static>>;
}

impl<'a, B: Backend> DescriptorInterface<'a, B> for BufferTypeless<'a, B> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
}

/*
impl<'a, B: Backend, T: StructuredBufferData> DescriptorInterface<'a, B> for HostReference<'a, B, T> {
    const TYPE: Option<&'static TypeDesc<'static>> = Some(<T as StructuredBufferData>::TYPE);
}*/

/*
impl<'a, B: Backend, T: StructuredBufferData> From<HostReference<'a, B, T>> for Descriptor<'a, B> {
    fn from(host: HostReference<'a, B, T>) -> Self {
        Descriptor::HostReference(host.0)
    }
}*/

// TODO: no impl for T: !BufferLayout, must use specialization
impl<'a, B: Backend, T: ?Sized + StructuredBufferData> DescriptorInterface<'a, B>
    for Buffer<'a, B, T>
{
    // T: BufferLayout so we have type info about the contents
    const TYPE: Option<&'static TypeDesc<'static>> = Some(<T as StructuredBufferData>::TYPE);
}

impl<'a, B: Backend> DescriptorInterface<'a, B> for TextureImageView<'a, B> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
}

impl<'a, B: Backend> DescriptorInterface<'a, B> for ImageView<'a, B> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
}

impl<'a, B: Backend> From<BufferTypeless<'a, B>> for Descriptor<'a, B> {
    fn from(buffer: BufferTypeless<'a, B>) -> Self {
        Descriptor::Buffer {
            buffer: buffer.0,
            offset: 0,
            size: None,
        }
    }
}

impl<'a, B: Backend, T: BufferData + ?Sized> From<Buffer<'a, B, T>> for Descriptor<'a, B> {
    fn from(buffer: Buffer<'a, B, T>) -> Self {
        // TODO pass/check type info?
        buffer.into_typeless().into()
    }
}

impl<'a, B: Backend> From<TextureImageView<'a, B>> for Descriptor<'a, B> {
    fn from(img: TextureImageView<'a, B>) -> Self {
        Descriptor::SampledImage {
            img: img.0,
            sampler: img.1,
        }
    }
}

impl<'a, B: Backend> From<ImageView<'a, B>> for Descriptor<'a, B> {
    fn from(img: ImageView<'a, B>) -> Self {
        Descriptor::Image {
            img: img.0
        }
    }
}

/*
impl<'a, B: Backend> DescriptorInterface<'a, B> for Image<'a, B> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
}*/

/*
impl<'a, B: Backend> From<Image<'a, B>> for Descriptor<'a, B> {
    fn from(img: Image<'a, B>) -> Self {
        Descriptor::Image { img: img.0 }
    }
}
*/
