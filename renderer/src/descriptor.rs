use crate::arena::BufferTypeless;
use crate::arena::Image;
use crate::image::SamplerDescription;
use crate::interface::TypeDesc;
use crate::shader::ShaderStageFlags;
use crate::traits::RendererBackend;
use derivative::Derivative;

#[derive(Copy, Clone, Debug)]
pub struct DescriptorSetLayoutBinding<'tcx> {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub stage_flags: ShaderStageFlags,
    pub count: usize,
    pub tydesc: Option<&'tcx TypeDesc<'tcx>>,
}

#[derive(Copy, Clone, Debug)]
pub enum DescriptorType {
    Sampler, // TODO
    SampledImage,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
    InputAttachment,
}

//--------------------------------------------------------------------------------------------------
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
