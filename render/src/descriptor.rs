//! Descriptors
use crate::arena::BufferTypeless;
use crate::arena::Image;
use crate::image::SamplerDescription;
use crate::interface::TypeDesc;
use crate::shader::ShaderStageFlags;
use crate::traits::RendererBackend;
use derivative::Derivative;

/// Represents an entry (binding) in a descriptor set layout.
#[derive(Copy, Clone, Debug)]
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
#[derive(Copy, Clone, Debug)]
pub enum DescriptorType {
    Sampler, // TODO
    SampledImage,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
    InputAttachment,
}

/// A reference to a resource used by one or more shader stages in the pipeline.
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""))]
pub enum Descriptor<'a, R: RendererBackend> {
    SampledImage {
        img: Image<'a, R>,
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
    Empty,
}

/*
// workaround lack of autocompletion for proc-macros
#[cfg(feature = "intellij")]
impl<'a, R: RendererBackend> Clone for Descriptor<'a,R> {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

#[cfg(feature = "intellij")]
impl<'a, R: RendererBackend> Copy for Descriptor<'a,R> {}
*/
