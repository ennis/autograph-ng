use super::{
    api::types::*,
    buffer::RawBuffer,
    image::RawImage,
    pipeline::{BindingSpace, DescriptorMap},
    pool::{BufferAliasKey, ImageAliasKey},
    resource::{Buffer, Image, SamplerCache},
    OpenGlBackend,
};
use crate::renderer::{Descriptor, DescriptorSetLayoutBinding, DescriptorType, ShaderStageFlags};

const MAX_INLINE_SHADER_RESOURCE_BINDINGS: usize = 16;

pub struct ShaderResourceBindings {
    /*pub textures: Vec<[GLuint; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,
    pub samplers: smallvec::SmallVec<[GLuint; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,
    pub images: smallvec::SmallVec<[GLuint; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,
    pub uniform_buffers: smallvec::SmallVec<[GLuint; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,
    pub uniform_buffer_sizes: smallvec::SmallVec<[GLintptr; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,
    pub uniform_buffer_offsets: smallvec::SmallVec<[GLintptr; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,
    pub shader_storage_buffers: smallvec::SmallVec<[GLuint; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,
    pub shader_storage_buffer_sizes:
        smallvec::SmallVec<[GLintptr; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,
    pub shader_storage_buffer_offsets:
        smallvec::SmallVec<[GLintptr; MAX_INLINE_SHADER_RESOURCE_BINDINGS]>,*/

    pub textures: Vec<GLuint>,
    pub samplers: Vec<GLuint>,
    pub images: Vec<GLuint>,
    pub uniform_buffers: Vec<GLuint>,
    pub uniform_buffer_sizes: Vec<GLintptr>,
    pub uniform_buffer_offsets: Vec<GLintptr>,
    pub shader_storage_buffers: Vec<GLuint>,
    pub shader_storage_buffer_sizes: Vec<GLintptr>,
    pub shader_storage_buffer_offsets: Vec<GLintptr>,
}

impl ShaderResourceBindings {
    pub fn new() -> ShaderResourceBindings {
        ShaderResourceBindings {
            /*textures: smallvec::SmallVec::new(),
            samplers: smallvec::SmallVec::new(),
            images: smallvec::SmallVec::new(),
            uniform_buffers: smallvec::SmallVec::new(),
            uniform_buffer_sizes: smallvec::SmallVec::new(),
            uniform_buffer_offsets: smallvec::SmallVec::new(),
            shader_storage_buffers: smallvec::SmallVec::new(),
            shader_storage_buffer_sizes: smallvec::SmallVec::new(),
            shader_storage_buffer_offsets: smallvec::SmallVec::new()*/
            textures: Vec::new(),
            samplers: Vec::new(),
            images: Vec::new(),
            uniform_buffers: Vec::new(),
            uniform_buffer_sizes: Vec::new(),
            uniform_buffer_offsets: Vec::new(),
            shader_storage_buffers: Vec::new(),
            shader_storage_buffer_sizes: Vec::new(),
            shader_storage_buffer_offsets: Vec::new()
        }
    }
}

#[derive(Debug)]
pub struct TypelessDescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub stage_flags: ShaderStageFlags,
    pub count: usize,
}

impl<'tcx> From<DescriptorSetLayoutBinding<'tcx>> for TypelessDescriptorSetLayoutBinding {
    fn from(b: DescriptorSetLayoutBinding<'tcx>) -> Self {
        TypelessDescriptorSetLayoutBinding {
            binding: b.binding,
            descriptor_type: b.descriptor_type,
            stage_flags: b.stage_flags,
            count: b.count,
        }
    }
}

#[derive(Debug)]
pub struct DescriptorSetLayout {
    pub bindings: Vec<TypelessDescriptorSetLayoutBinding>,
}

/// Backend version of descriptors. Cannot contain borrows because of the lack of ATCs, so
/// directly store OpenGL objects and rely on the renderer wrapper to statically check the lifetimes
/// for us.
#[derive(Debug)]
pub enum RawDescriptor {
    Image {
        image: GLuint,
    },
    Texture {
        image: GLuint,
        sampler: GLuint,
    },
    UniformBuffer {
        buffer: GLuint,
        offset: usize,
        size: usize,
    },
    StorageBuffer {
        buffer: GLuint,
        offset: usize,
        size: usize,
    },
}

#[derive(Debug)]
pub struct DescriptorSet {
    pub descriptors: Vec<RawDescriptor>,
}

impl DescriptorSet {
    pub fn from_descriptors_and_layout(
        descriptors: &[Descriptor<OpenGlBackend>],
        layout: &DescriptorSetLayout,
        sampler_cache: &mut SamplerCache,
    ) -> DescriptorSet {
        DescriptorSet {
            descriptors: descriptors
                .iter()
                .enumerate()
                .map(|(i, d)| match d {
                    Descriptor::SampledImage { img, sampler } => {
                        match layout.bindings[i].descriptor_type {
                            DescriptorType::SampledImage => RawDescriptor::Texture {
                                image: img.obj,
                                sampler: sampler_cache.get_sampler(sampler),
                            },
                            _ => panic!("unexpected descriptor type"),
                        }
                    }
                    Descriptor::Image { img } => match layout.bindings[i].descriptor_type {
                        DescriptorType::StorageImage => RawDescriptor::Image { image: img.obj },
                        _ => panic!("unexpected descriptor type"),
                    },
                    Descriptor::Buffer {
                        buffer,
                        offset,
                        size,
                    } => match layout.bindings[i].descriptor_type {
                        DescriptorType::StorageBuffer => RawDescriptor::StorageBuffer {
                            buffer: buffer.obj,
                            offset: buffer.offset + *offset,
                            size: *size,
                        },
                        DescriptorType::UniformBuffer => RawDescriptor::UniformBuffer {
                            buffer: buffer.obj,
                            offset: buffer.offset + *offset,
                            size: *size,
                        },
                        _ => panic!("unexpected descriptor type"),
                    },
                    Descriptor::Empty => panic!("unexpected empty descriptor"),
                })
                .collect(),
        }
    }

    pub fn collect(
        &self,
        this_set_index: u32,
        map: &DescriptorMap,
        sr: &mut ShaderResourceBindings,
    ) {
        /*fn bind<A: smallvec::Array>(
            v: &mut smallvec::SmallVec<A>,
            index: usize,
            item: A::Item,
            default: A::Item,
        ) where
            A::Item: Copy,
        {
            if index >= v.len() {
                v.resize(index + 1, default);
            }
            v[index] = item;
        }*/

        fn bind<T>(
            v: &mut Vec<T>,
            index: usize,
            item: T,
            default: T,
        ) where
            T: Copy,
        {
            if index >= v.len() {
                v.resize(index + 1, default);
            }
            v[index] = item;
        }

        fn check_descriptor_type(ty: BindingSpace, expected: BindingSpace) {
            if ty != expected {
                panic!(
                    "descriptor binding spaces do not match (expected: {:?}; got {:?})",
                    expected, ty
                )
            }
        }

        for (i, d) in self.descriptors.iter().enumerate() {
            let loc = map
                .get_binding_location(this_set_index, i as u32)
                .expect("descriptor (set,binding) is not mapped to any OpenGL binding point");

            match d {
                RawDescriptor::UniformBuffer {
                    buffer,
                    offset,
                    size,
                } => {
                    check_descriptor_type(loc.space, BindingSpace::UniformBuffer);
                    bind(&mut sr.uniform_buffers, loc.location as usize, *buffer, 0);
                    bind(
                        &mut sr.uniform_buffer_offsets,
                        loc.location as usize,
                        *offset as isize,
                        0,
                    );
                    bind(&mut sr.uniform_buffer_sizes, loc.location as usize, *size as isize, 0);
                }
                RawDescriptor::StorageBuffer {
                    buffer,
                    offset,
                    size,
                } => {
                    check_descriptor_type(loc.space, BindingSpace::ShaderStorageBuffer);
                    bind(
                        &mut sr.shader_storage_buffers,
                        loc.location as usize,
                        *buffer,
                        0,
                    );
                    bind(
                        &mut sr.shader_storage_buffer_offsets,
                        loc.location as usize,
                        *offset as isize,
                        0,
                    );
                    bind(
                        &mut sr.shader_storage_buffer_sizes,
                        loc.location as usize,
                        *size as isize,
                        0,
                    );
                }
                RawDescriptor::Texture { image, sampler } => {
                    check_descriptor_type(loc.space, BindingSpace::Texture);
                    bind(&mut sr.textures, loc.location as usize, *image, 0);
                    bind(&mut sr.samplers, loc.location as usize, *sampler, 0);
                }
                RawDescriptor::Image { image } => {
                    check_descriptor_type(loc.space, BindingSpace::Image);
                    bind(&mut sr.images, loc.location as usize, *image, 0);
                }
            }
        }
    }
}
