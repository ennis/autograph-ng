//! Descriptors
use crate::buffer::Buffer;
use crate::buffer::BufferData;
use crate::buffer::BufferTypeless;
use crate::buffer::StructuredBufferData;
use crate::image::SampledImage;
use crate::image::SamplerDescription;
use crate::pipeline::ShaderStageFlags;
use crate::handle;
use crate::typedesc::TypeDesc;
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct HostReference<'a, T: BufferData>(pub handle::HostReference<'a>, pub(crate) PhantomData<&'a T>);

/// Represents an entry (binding) in a descriptor set layout.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DescriptorBinding<'tcx> {
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
        img: handle::Image<'a>,
        sampler: SamplerDescription,
    },
    Image {
        img: handle::Image<'a>,
    },
    Buffer {
        buffer: handle::Buffer<'a>,
        offset: usize,
        size: Option<usize>,
    },
    HostReference(handle::HostReference<'a>),
    Empty,
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

impl<'a, T: StructuredBufferData> DescriptorInterface<'a> for HostReference<'a, T> {
    const TYPE: Option<&'static TypeDesc<'static>> = Some(<T as StructuredBufferData>::TYPE);
}

impl<'a, T: StructuredBufferData> From<HostReference<'a, T>> for Descriptor<'a> {
    fn from(host: HostReference<'a, T>) -> Self {
        Descriptor::HostReference(host.0)
    }
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
        Descriptor::Buffer {
            buffer: buffer.0,
            offset: 0,
            size: None,
        }
    }
}

impl<'a, T: BufferData + ?Sized> From<Buffer<'a, T>> for Descriptor<'a> {
    fn from(buffer: Buffer<'a, T>) -> Self {
        // TODO pass/check type info?
        buffer.into_typeless().into()
    }
}

impl<'a, 'b> From<SampledImage<'a>> for Descriptor<'a> {
    fn from(img: SampledImage<'a>) -> Self {
        Descriptor::SampledImage {
            img: img.0,
            sampler: img.1,
        }
    }
}
