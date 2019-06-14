//! Descriptors
use crate::{
    buffer::BufferData, format::Format, image::SamplerDescription, pipeline::ShaderStageFlags,
    typedesc::TypeDesc, Backend,
};
use autograph_spirv::layout::Layout;
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
    /// Set index. `None` means inferred from the argument structure, or irrelevant.
    pub set: Option<u32>,
    /// Binding index
    pub index: u32,
    /// Descriptor type
    pub ty: ResourceBindingType,
    /// Which shader stages will see this descriptor
    pub stage_flags: ShaderStageFlags,
    /// TODO How many descriptors in the binding? Should be 1
    pub count: u32,
    /// Precise description of the expected data type (image format).
    ///
    /// Can be None if no type information is available for this binding.
    pub data_ty: Option<&'tcx TypeDesc<'tcx>>,
    /// Data layout.
    pub data_layout: Option<&'tcx Layout<'tcx>>,
    /// Data format for r/w images & texel buffers.
    /// `Format::UNDEFINED` if not applicable (all other binding types)
    pub data_format: Format,
}

#[derive(Copy, Clone, Debug)]
pub struct SubresourceRange {
    pub base_mip_level: u32,
    pub level_count: Option<u32>,
    pub base_array_layer: u32,
    pub layer_count: Option<u32>,
}

/// A reference to a resource used by one or more shader stages in the pipeline.
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
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
    const DATA_TYPE: Option<&'static TypeDesc<'static>> = None;
    const DATA_LAYOUT: Option<&'static Layout<'static>> = None;
    const DATA_FORMAT: Format = Format::UNDEFINED;
    fn into_descriptor(self) -> Descriptor<'a, B>;
}
