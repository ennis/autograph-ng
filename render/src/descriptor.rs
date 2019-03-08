//! Descriptors
use crate::{
    buffer::BufferData, image::SamplerDescription, pipeline::ShaderStageFlags, typedesc::TypeDesc,
    Backend,
};
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct HostReference<'a, B: Backend, T: BufferData>(
    pub &'a B::HostReference,
    pub(crate) PhantomData<&'a T>,
);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ResourceShape {
    R1d,
    R1dArray,
    R2d,
    R2dArray,
    R2dMultisample,
    R2dMultisampleArray,
    R3d,
    RCube,
}

/// Descriptor type
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ResourceBindingType {
    Sampler,
    Texture(ResourceShape),
    TextureSampler(ResourceShape),
    //Image(ResourceShape), => use texture instead
    RwImage(ResourceShape),
    ConstantBuffer,
    RwBuffer,
    TexelBuffer,
    RwTexelBuffer,
}

///
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ResourceBinding<'tcx> {
    /// Binding index
    pub index: usize,
    /// Descriptor type
    pub ty: ResourceBindingType,
    /// Which shader stages will see this descriptor
    pub stage_flags: ShaderStageFlags,
    /// TODO How many descriptors in the binding? Should be 1
    pub count: usize,
    /// Precise description of the expected data type (image format, layout of buffer data, etc.).
    ///
    /// Can be None if no type information is available for this binding.
    pub data_ty: Option<&'tcx TypeDesc<'tcx>>,
}

#[derive(Copy, Clone, Debug)]
pub struct SubresourceRange {
    pub base_mip_level: u32,
    pub level_count: Option<u32>,
    pub base_array_layer: u32,
    pub layer_count: Option<u32>,
}

/// A reference to a resource used by one or more shader stages in the pipeline.
#[derive(Copy, Clone)]
pub enum Descriptor<'a, B: Backend> {
    Sampler {
        desc: SamplerDescription,
    },
    Texture {
        image: &'a B::Image,
        subresource: SubresourceRange,
    },
    TextureSampler {
        image: &'a B::Image,
        subresource: SubresourceRange,
        sampler: SamplerDescription,
    },
    RwImage {
        image: &'a B::Image,
        subresource: SubresourceRange,
    },
    ConstantBuffer {
        buffer: &'a B::Buffer,
        offset: usize,
        size: Option<usize>,
    },
    RwBuffer {
        buffer: &'a B::Buffer,
        offset: usize,
        size: Option<usize>,
    },
    TexelBuffer {
        buffer: &'a B::Buffer,
        offset: usize,
        size: Option<usize>,
    },
    RwTexelBuffer {
        buffer: &'a B::Buffer,
        offset: usize,
        size: Option<usize>,
    },
    Empty,
}

/// Trait implemented by types that can be turned into descriptors.
///
/// This trait is implemented by default for buffer objects, buffer slices, and images.
pub trait ResourceInterface<'a, B: Backend> {
    /// Descriptor type
    const TYPE: ResourceBindingType;
    /// Type information about the content of the data referenced by the descriptor.
    const DATA_TYPE: Option<&'static TypeDesc<'static>>;
    fn into_descriptor(self) -> Descriptor<'a, B>;
}

/*impl<'a, B: Backend> ResourceInterface<'a, B> for BufferTypeless<'a, B> {
    const TYPE: ResourceBindingType = ResourceBindingType::;
    const DATA_TYPE: Option<&'static TypeDesc<'static>> = None;
}*/

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
/*
impl<'a, B: Backend, T: ?Sized + StructuredBufferData> ResourceInterface<'a, B>
    for Buffer<'a, B, T>
{
    // T: BufferLayout so we have type info about the contents
    const TYPE: Option<&'static TypeDesc<'static>> = Some(<T as StructuredBufferData>::TYPE);
}

impl<'a, B: Backend> ResourceInterface<'a, B> for TextureImageView<'a, B> {
    const TYPE: Option<&'static TypeDesc<'static>> = None;
}

impl<'a, B: Backend> ResourceInterface<'a, B> for ImageView<'a, B> {
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
}*/
/*
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
}*/

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
