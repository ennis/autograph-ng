use crate::api::types::*;
use crate::api::Gl;
use crate::backend::GlArena;
use crate::command::StateCache;
use autograph_render::image::SamplerDescription;
use autograph_render::pipeline::ColorBlendAttachmentState;
use autograph_render::pipeline::ColorBlendAttachments;
use autograph_render::pipeline::DepthStencilState;
use autograph_render::pipeline::GraphicsPipelineCreateInfoTypeless;
use autograph_render::pipeline::InputAssemblyState;
use autograph_render::pipeline::LogicOp;
use autograph_render::pipeline::MultisampleState;
use autograph_render::pipeline::RasterisationState;
use autograph_render::pipeline::VertexInputBindingDescription;
use ordered_float::NotNan;

mod program;
mod shader;
mod vao;

use self::program::create_graphics_program;
use self::vao::create_vertex_array_object;

pub(crate) use self::shader::BindingSpace;
pub(crate) use self::shader::DescriptorMap;
pub(crate) use self::shader::GlShaderModule;
use crate::HandleCast;
use autograph_render::pipeline::Viewport;
use autograph_render::pipeline::ScissorRect;
use autograph_render::pipeline::PipelineSignatureDescription;
use autograph_render::descriptor::DescriptorType;
use autograph_render::vertex::IndexFormat;
use autograph_render::pipeline::PipelineArgumentsBuilder;
use autograph_render::pipeline::PipelineArgumentsCreateInfoTypeless;
use crate::backend::PipelineSignatureCache;
use autograph_render::descriptor::Descriptor;
use crate::image::GlImage;
use crate::buffer::GlBuffer;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct StaticSamplerEntry {
    pub(crate) tex_range: (u32, u32),
    pub(crate) desc: SamplerDescription,
}

//--------------------------------------------------------------------------------------------------
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum PipelineColorBlendAttachmentsOwned {
    All(ColorBlendAttachmentState),
    Separate(Vec<ColorBlendAttachmentState>),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct PipelineColorBlendStateOwned {
    pub(crate) logic_op: Option<LogicOp>,
    pub(crate) attachments: PipelineColorBlendAttachmentsOwned,
    pub(crate) blend_constants: [NotNan<f32>; 4],
}

#[derive(Clone, Debug)]
pub(crate) struct GlGraphicsPipeline {
    pub(crate) rasterization_state: RasterisationState,
    pub(crate) depth_stencil_state: DepthStencilState,
    pub(crate) multisample_state: MultisampleState,
    pub(crate) input_assembly_state: InputAssemblyState,
    pub(crate) vertex_input_bindings: Vec<VertexInputBindingDescription>,
    pub(crate) color_blend_state: PipelineColorBlendStateOwned,
    pub(crate) descriptor_map: DescriptorMap,
    pub(crate) program: GLuint,
    pub(crate) vao: GLuint,
}

impl GlGraphicsPipeline {
    pub fn descriptor_map(&self) -> &DescriptorMap {
        &self.descriptor_map
    }

    pub fn vertex_input_bindings(&self) -> &[VertexInputBindingDescription] {
        &self.vertex_input_bindings
    }
}

//--------------------------------------------------------------------------------------------------
pub(crate) unsafe fn create_graphics_pipeline_internal<'a>(
    gl: &Gl,
    arena: &'a GlArena,
    ci: &GraphicsPipelineCreateInfoTypeless,
) -> &'a GlGraphicsPipeline {
    let (program, descriptor_map) = {
        let vs = ci
            .shader_stages
            .vertex
            .0
            .cast();
        let fs = ci
            .shader_stages
            .fragment
            .map(|s| s.0.cast());
        let gs = ci
            .shader_stages
            .geometry
            .map(|s| s.0.cast());
        let tcs = ci
            .shader_stages
            .tess_control
            .map(|s| s.0.cast());
        let tes = ci
            .shader_stages
            .tess_eval
            .map(|s| s.0.cast());
        create_graphics_program(gl, vs, fs, gs, tcs, tes).expect("failed to create program")
    };

    //assert_eq!(vertex_shader.stage, ShaderStageFlags::VERTEX);
    let vao = create_vertex_array_object(gl, ci.vertex_input_state.attributes);

    let color_blend_state = PipelineColorBlendStateOwned {
        logic_op: ci.color_blend_state.logic_op,
        attachments: match ci.color_blend_state.attachments {
            ColorBlendAttachments::All(a) => PipelineColorBlendAttachmentsOwned::All(*a),
            ColorBlendAttachments::Separate(a) => {
                PipelineColorBlendAttachmentsOwned::Separate(a.to_vec())
            }
        },
        blend_constants: ci.color_blend_state.blend_constants,
    };

    let g = GlGraphicsPipeline {
        rasterization_state: *ci.rasterization_state,
        depth_stencil_state: *ci.depth_stencil_state,
        multisample_state: *ci.multisample_state,
        input_assembly_state: *ci.input_assembly_state,
        vertex_input_bindings: ci.vertex_input_state.bindings.to_vec(),
        program,
        vao,
        descriptor_map,
        color_blend_state,
    };

    arena.graphics_pipelines.alloc(g)
}

impl GlGraphicsPipeline {
    pub(crate) fn bind(&self, gl: &Gl, state_cache: &mut StateCache) {
        state_cache.set_program(gl, self.program);
        state_cache.set_vertex_array(gl, self.vao);
        state_cache.set_cull_mode(gl, self.rasterization_state.cull_mode);
        state_cache.set_polygon_mode(gl, self.rasterization_state.polygon_mode);
        state_cache.set_stencil_test(gl, &self.depth_stencil_state.stencil_test);
        state_cache.set_depth_test_enable(gl, self.depth_stencil_state.depth_test_enable);
        state_cache.set_depth_write_enable(gl, self.depth_stencil_state.depth_write_enable);
        state_cache.set_depth_compare_op(gl, self.depth_stencil_state.depth_compare_op);
        match self.color_blend_state.attachments {
            PipelineColorBlendAttachmentsOwned::All(ref state) => {
                state_cache.set_all_blend(gl, state)
            }
            PipelineColorBlendAttachmentsOwned::Separate(ref states) => {
                for (i, s) in states.iter().enumerate() {
                    state_cache.set_blend_separate(gl, i as u32, s);
                }
            }
        }
    }
}

#[derive(Copy,Clone,Debug)]
pub(crate) struct GlPipelineSignature<'a>
{
    pub(crate) sub_signatures: &'a [&'a GlPipelineSignature<'a>],
    // descriptor #n -> binding space
    pub(crate) descriptor_map: &'a [DescriptorType],
    pub(crate) num_vertex_buffers: usize,
    pub(crate) num_uniform_buffers: usize,
    pub(crate) num_shader_storage_buffers: usize,
    pub(crate) num_textures: usize,
    pub(crate) num_images: usize,
    pub(crate) num_render_targets: usize,
    pub(crate) has_index_buffer: bool,
    pub(crate) has_depth_render_target: bool,
    pub(crate) is_root_fragment_output_signature: bool,
    pub(crate) is_root_vertex_input_signature: bool,
}

impl<'a> GlPipelineSignature<'a>
{
    pub(crate) fn new<'r: 'a>(
        arena: &'a GlArena,
        cache: &'r PipelineSignatureCache,
        create_info: &PipelineSignatureDescription) -> GlPipelineSignature<'a>
    {
        // TODO allocate directly in arena when alloc_extend is implemented
        let sub_signatures = arena.other.alloc_extend(create_info.sub_signatures.iter().map(|sig| {
            GlPipelineSignature::get_or_create(arena, cache, create_info)
        }));

        let descriptor_map = arena.other.alloc_extend(create_info.descriptors.iter().map(|d| d.descriptor_type));

        // count number of bindings of each type
        //let mut num_vertex_buffers = 0;
        //let mut has_index_buffer = 0;
        let mut num_uniform_buffers = 0;
        let mut num_shader_storage_buffers = 0;
        let mut num_input_attachments = 0;
        let mut num_textures = 0;
        let mut num_images = 0;
        let mut num_samplers = 0;
        //let mut num_render_targets = 0;
        for d in descriptor_map.iter() {
            match d {
                DescriptorType::SampledImage => num_textures += 1,
                DescriptorType::StorageImage => num_images += 1,
                DescriptorType::UniformBuffer => num_uniform_buffers += 1,
                DescriptorType::StorageBuffer => num_shader_storage_buffers += 1,
                DescriptorType::InputAttachment => num_input_attachments += 1,
                DescriptorType::Sampler => num_samplers += 1,
            }
        }

        GlPipelineSignature {
            sub_signatures,
            descriptor_map,
            num_vertex_buffers: create_info.vertex_layouts.len(),
            has_index_buffer: create_info.index_format.is_some(),
            has_depth_render_target: create_info.depth_stencil_fragment_output.is_some(),
            num_uniform_buffers,
            num_shader_storage_buffers,
            num_textures,
            num_images,
            num_render_targets: create_info.fragment_outputs.len(),
            is_root_fragment_output_signature: create_info.is_root_fragment_output_signature,
            is_root_vertex_input_signature: create_info.is_root_vertex_input_signature
        }
    }

    pub(crate) fn get_or_create<'r: 'a>(
        arena: &'a GlArena,
        cache: &'r PipelineSignatureCache,
        create_info: &PipelineSignatureDescription) -> &'a GlPipelineSignature<'a>
    {
        if let Some(typeid) = create_info.typeid {
            if let Some(sig) = cache.get(typeid) {
                sig
            } else {
                let sig = GlPipelineSignature::new(arena, cache, create_info);
                cache.get_or_insert_with(typeid, || sig)
            }
        } else {
            arena.other.alloc(GlPipelineSignature::new(arena, cache, create_info))
        }
    }
}

const MAX_INLINE_ARGUMENTS: usize = 8;

pub(crate) struct GlIndexBuffer
{
    buffer: GLuint,
    offset: GLintptr,
    fmt: IndexFormat
}

/*
impl<'a> PipelineArgumentsBuilder<'a> for GlPipelineArgumentsBuilder<'a>
{
    unsafe fn push_arguments(&mut self, arguments: handle::PipelineArguments<'a>) {
        let arguments: &GlPipelineArguments = arguments.downcast_ref_unwrap();
        self.args.push(arguments);
    }

    unsafe fn push_descriptor(&mut self, descriptor: Descriptor<'a>) {
        let ty = self.signature.descriptor_map[self.descriptor_index];
        match ty {
            DescriptorType::SampledImage => {
                match descriptor {
                    Descriptor::SampledImage { img, ref sampler } => {
                        let img: &GlImage = img.downcast_ref_unwrap();
                        self.textures.push(img.raw.obj);
                        self.samplers.push(self.sampler_cache.get_sampler(gl, sampler));
                    },
                    _ => panic!("unexpected descriptor type")
                }
            },
            DescriptorType::StorageImage => {
                match descriptor {
                    Descriptor::Image { img } => {
                        let img: &GlImage = img.downcast_ref_unwrap();
                        self.images.push(img.raw.obj);
                    },
                    _ => panic!("unexpected descriptor type")
                }
            },
            DescriptorType::UniformBuffer => {
                match descriptor {
                    Descriptor::Buffer {
                        buffer,
                        offset,
                        size, } => {
                        let buffer: &GlBuffer = buffer.downcast_ref_unwrap();
                        self.uniform_buffers.push(buffer.raw.obj);
                        self.uniform_buffer_offsets.push(buffer.offset + offset);
                        self.uniform_buffer_sizes.push(size);
                    },
                    _ => panic!("unexpected descriptor type")
                }

            },
            DescriptorType::StorageBuffer => {
                match descriptor {
                    Descriptor::Buffer {
                        buffer,
                        offset,
                        size, } => {
                        let buffer: &GlBuffer = buffer.downcast_ref_unwrap();
                        self.shader_storage_buffers.push(buffer.raw.obj);
                        self.shader_storage_buffer_offsets.push(buffer.offset + offset);
                        self.shader_storage_buffer_sizes.push(size);
                    },
                    _ => panic!("unexpected descriptor type")
                }
            },
            DescriptorType::InputAttachment => {
                unimplemented!()
            },
            DescriptorType::Sampler => {
                unimplemented!()
            },
        }
    }

    unsafe fn push_viewport(&mut self, viewport: &Viewport) {
        self.viewports.push(viewport);
    }

    unsafe fn push_scissor(&mut self, scissor: &ScissorRect) {
        self.scissors.push(scissor);
    }

    unsafe fn push_vertex_buffer(&mut self, vertex_buffer: VertexBufferDescriptor<'a, '_>) {
        let buffer: &GlBuffer = vertex_buffer.buffer.0.downcast_ref_unwrap();
        self.vertex_buffers.push(buffer.raw.obj);
        self.vertex_buffer_offsets.push(buffer.offset + vertex_buffer.offset);
        self.vertex_buffer_strides.push(vertex_buffer.layout.stride);
    }

    unsafe fn push_index_buffer(&mut self, index_buffer: IndexBufferDescriptor<'a>) {
        let buffer: &GlBuffer = index_buffer.buffer.0.downcast_ref_unwrap();
        self.index_buffer = Some(
            GlIndexBuffer {
                buffer: buffer.raw.obj,
                offset: buffer.offset + index_buffer.offset,
                fmt: index_buffer.format
            });
    }

    unsafe fn push_render_target(&mut self, render_target: RenderTargetDescriptor<'a>) {
        let image: &GlImage = render_target.image.0.downcast_ref_unwrap();
        self.render_targets.push(image);
    }

    unsafe fn push_depth_stencil_render_target(&mut self, depth_stencil_render_target: RenderTargetDescriptor<'a>) {
        let image: &GlImage = render_target.image.0.downcast_ref_unwrap();
        self.depth_stencil_render_target = Some(image);
    }
}*/

#[derive(Copy,Clone,Debug)]
pub(crate) struct GlPipelineArguments<'a> {
    pub(crate) signature: &'a GlPipelineSignature<'a>,
    pub(crate) blocks: &'a [StateBlock<'a>]
}

impl<'a> GlPipelineArguments<'a> {
    pub(crate) fn new(arena: &'a GlArena, create_info: &PipelineArgumentsCreateInfoTypeless<'a, '_>) -> GlPipelineArguments<'a> {

        let signature : &GlPipelineSignature = create_info.signature.0.cast();

        let uniform_buffers = unsafe { arena.other.alloc_uninitialized(signature.num_uniform_buffers) };
        let uniform_buffers_offsets = unsafe { arena.other.alloc_uninitialized(signature.num_uniform_buffers) };
        let uniform_buffers_sizes = unsafe { arena.other.alloc_uninitialized(signature.num_uniform_buffers) };

        let shader_storage_buffers = unsafe { arena.other.alloc_uninitialized(signature.num_shader_storage_buffers) };
        let shader_storage_buffers_offsets = unsafe { arena.other.alloc_uninitialized(signature.num_shader_storage_buffers) };
        let shader_storage_buffers_sizes = unsafe { arena.other.alloc_uninitialized(signature.num_shader_storage_buffers) };

        let vertex_buffers = unsafe { arena.other.alloc_uninitialized(signature.num_vertex_buffers) };
        let vertex_buffers_offsets = unsafe { arena.other.alloc_uninitialized(signature.num_vertex_buffers) };
        let vertex_buffers_strides = unsafe { arena.other.alloc_uninitialized(signature.num_vertex_buffers) };

        let textures = unsafe { arena.other.alloc_uninitialized(signature.num_textures) };
        let images = unsafe { arena.other.alloc_uninitialized(signature.num_images) };
        let samplers = unsafe { arena.other.alloc_uninitialized(signature.num_samplers) };

        let mut i_uniform = 0;
        let mut i_storage = 0;
        let mut i_texture = 0;
        let mut i_image = 0;
        for (i,b) in create_info.descriptors.iter().enumerate() {
            let ty = signature.descriptor_map[i];
            match ty {
                DescriptorType::SampledImage => {
                    match descriptor {
                        Descriptor::SampledImage { img, ref sampler } => {
                            let img: &GlImage = img.cast();
                            *textures[i_texture] = img.raw.obj;
                            *samplers[i_texture] = sampler_cache.get_sampler(gl, sampler);
                            i_texture += 1;
                        },
                        _ => panic!("unexpected descriptor type")
                    }
                },
                DescriptorType::StorageImage => {
                    match descriptor {
                        Descriptor::Image { img } => {
                            let img: &GlImage = img.cast();
                            *images[i_image] = img.raw.obj;
                            i_image += 1;
                        },
                        _ => panic!("unexpected descriptor type")
                    }
                },
                DescriptorType::UniformBuffer => {
                    match descriptor {
                        Descriptor::Buffer {
                            buffer,
                            offset,
                            size, } => {
                            let buffer: &GlBuffer = buffer.cast();
                            uniform_buffers[i_uniform] = buffer.raw.obj;
                            uniform_buffer_offsets[i_uniform] = buffer.offset + offset;
                            uniform_buffer_sizes[i_uniform] = size;
                            i_uniform += 1;
                        },
                        _ => panic!("unexpected descriptor type")
                    }

                },
                DescriptorType::StorageBuffer => {
                    match descriptor {
                        Descriptor::Buffer {
                            buffer,
                            offset,
                            size, } => {
                            let buffer: &GlBuffer = buffer.cast();
                            shader_storage_buffers[i_uniform] = buffer.raw.obj;
                            shader_storage_buffer_offsets[i_uniform] = buffer.offset + offset;
                            shader_storage_buffer_sizes[i_uniform] = size;
                            i_uniform += 1;
                        },
                        _ => panic!("unexpected descriptor type")
                    }
                },
                DescriptorType::InputAttachment => {
                    unimplemented!()
                },
                DescriptorType::Sampler => {
                    unimplemented!()
                },
            }
        }


        //let viewports = unsafe { arena.other.alloc_uninitialized(signature.num_) };
        //let scissors = unsafe { arena.other.alloc_uninitialized(signature.num_samplers) };
        //uniform_buffers.
    }
}

#[derive(Copy,Clone,Debug)]
pub(crate) enum StateBlock<'a> {
    Inherited(&'a [GlPipelineArguments<'a>]),
    UniformBuffers {
        buffers: &'a [GLuint],
        offsets: &'a [GLintptr],
        sizes: &'a [GLintptr],
    },
    ShaderStorageBuffers {
        buffers: &'a [GLuint],
        offsets: &'a [GLintptr],
        sizes: &'a [GLintptr],
    },
    VertexBuffers {
        buffers: &'a [GLuint],
        offsets: &'a [GLintptr],
        strides: &'a [GLintptr],
    },
    IndexBuffer(GLuint),
    Textures(&'a [GLuint]),
    Images(&'a [GLuint]),
    Samplers(&'a [GLuint]),
    RenderTarget(&'a [GLuint]),
    DepthStencilRenderTarget(GLuint),
    Framebuffer(GLuint),
    Viewports(&'a [Viewport]),
    Scissors(&'a [ScissorRect]),
}
