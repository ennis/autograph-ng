use crate::api::types::*;
use crate::api::Gl;
use crate::backend::GlArena;
use crate::backend::OpenGlBackend;
use crate::backend::PipelineSignatureCache;
use crate::buffer::GlBuffer;
use crate::command::StateCache;
use crate::image::GlImage;
use autograph_render::descriptor::Descriptor;
use autograph_render::descriptor::DescriptorType;
use autograph_render::framebuffer::RenderTargetDescriptor;
use autograph_render::image::SamplerDescription;
use autograph_render::pipeline::ColorBlendAttachmentState;
use autograph_render::pipeline::ColorBlendAttachments;
use autograph_render::pipeline::DepthStencilState;
use autograph_render::pipeline::GraphicsPipelineCreateInfoTypeless;
use autograph_render::pipeline::InputAssemblyState;
use autograph_render::pipeline::LogicOp;
use autograph_render::pipeline::MultisampleState;
use autograph_render::pipeline::PipelineArgumentsTypeless;
use autograph_render::pipeline::PipelineSignatureDescription;
use autograph_render::pipeline::RasterisationState;
use autograph_render::pipeline::ScissorRect;
use autograph_render::pipeline::VertexInputBindingDescription;
use autograph_render::pipeline::Viewport;
use autograph_render::vertex::IndexBufferDescriptor;
use autograph_render::vertex::IndexFormat;
use autograph_render::vertex::VertexBufferDescriptor;
use ordered_float::NotNan;
use std::iter;

mod program;
mod shader;
mod vao;

use self::program::create_graphics_program;
use self::vao::create_vertex_array_object;

pub(crate) use self::shader::DescriptorMap;
pub(crate) use self::shader::GlShaderModule;
use crate::framebuffer::GlFramebuffer;
use crate::sampler::SamplerCache;

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
pub struct GlGraphicsPipeline {
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
    pub(crate) fn descriptor_map(&self) -> &DescriptorMap {
        &self.descriptor_map
    }

    pub(crate) fn vertex_input_bindings(&self) -> &[VertexInputBindingDescription] {
        &self.vertex_input_bindings
    }
}

//--------------------------------------------------------------------------------------------------
pub(crate) unsafe fn create_graphics_pipeline_internal<'a>(
    gl: &Gl,
    arena: &'a GlArena,
    ci: &GraphicsPipelineCreateInfoTypeless<'a, '_, OpenGlBackend>,
) -> &'a GlGraphicsPipeline {
    let (program, descriptor_map) = {
        let vs = ci.shader_stages.vertex.0;
        let fs = ci.shader_stages.fragment.map(|s| s.0);
        let gs = ci.shader_stages.geometry.map(|s| s.0);
        let tcs = ci.shader_stages.tess_control.map(|s| s.0);
        let tes = ci.shader_stages.tess_eval.map(|s| s.0);
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

#[derive(Copy, Clone, Debug)]
pub struct GlPipelineSignature<'a> {
    pub(crate) sub_signatures: &'a [&'a GlPipelineSignature<'a>],
    // descriptor #n -> binding space
    pub(crate) descriptor_map: &'a [DescriptorType],
    pub(crate) num_state_blocks: usize,
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

impl<'a> GlPipelineSignature<'a> {
    pub(crate) fn new<'r: 'a>(
        arena: &'a GlArena,
        cache: &'r PipelineSignatureCache,
        create_info: &PipelineSignatureDescription,
    ) -> GlPipelineSignature<'a> {
        // TODO allocate directly in arena when alloc_extend is implemented
        let sub_signatures = create_info
            .sub_signatures
            .iter()
            .map(|&sig| GlPipelineSignature::get_or_create(arena, cache, sig))
            .collect::<Vec<_>>();
        let sub_signatures = arena.other.alloc_extend(sub_signatures);

        let descriptor_map = arena
            .other
            .alloc_extend(create_info.descriptors.iter().map(|d| d.descriptor_type));

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
        let num_vertex_buffers = create_info.vertex_layouts.len();
        let has_index_buffer = create_info.index_format.is_some();
        let num_render_targets = create_info.fragment_outputs.len();
        let has_depth_render_target = create_info.depth_stencil_fragment_output.is_some();

        let mut num_state_blocks = 0;
        if num_textures > 0 {
            num_state_blocks += 1;
        }
        if num_images > 0 {
            num_state_blocks += 1;
        }
        if num_uniform_buffers > 0 {
            num_state_blocks += 1;
        }
        if num_shader_storage_buffers > 0 {
            num_state_blocks += 1;
        }
        if num_vertex_buffers > 0 {
            num_state_blocks += 1;
        }
        if has_index_buffer {
            num_state_blocks += 1;
        }
        if create_info.is_root_fragment_output_signature {
            // will create framebuffer directly
            num_state_blocks += 1;
        } else {
            if num_render_targets > 0 {
                num_state_blocks += 1;
            }
            if has_depth_render_target {
                num_state_blocks += 1;
            }
        }
        // viewports & scissors
        num_state_blocks += 2;

        GlPipelineSignature {
            sub_signatures,
            descriptor_map,
            num_state_blocks,
            num_vertex_buffers,
            has_index_buffer,
            has_depth_render_target,
            num_uniform_buffers,
            num_shader_storage_buffers,
            num_textures,
            num_images,
            num_render_targets,
            is_root_fragment_output_signature: create_info.is_root_fragment_output_signature,
            is_root_vertex_input_signature: create_info.is_root_vertex_input_signature,
        }
    }

    pub(crate) fn get_or_create<'r: 'a>(
        arena: &'a GlArena,
        cache: &'r PipelineSignatureCache,
        create_info: &PipelineSignatureDescription,
    ) -> &'a GlPipelineSignature<'a> {
        if let Some(typeid) = create_info.typeid {
            if let Some(sig) = cache.get(typeid) {
                sig
            } else {
                let sig = GlPipelineSignature::new(arena, cache, create_info);
                cache.get_or_insert_with(typeid, || sig)
            }
        } else {
            arena
                .other
                .alloc(GlPipelineSignature::new(arena, cache, create_info))
        }
    }
}

const MAX_INLINE_ARGUMENTS: usize = 8;

pub(crate) struct GlIndexBuffer {
    buffer: GLuint,
    offset: GLintptr,
    fmt: IndexFormat,
}

#[derive(Copy, Clone, Debug)]
pub struct GlPipelineArguments<'a> {
    pub(crate) signature: &'a GlPipelineSignature<'a>,
    pub(crate) blocks: &'a [StateBlock<'a>],
}

impl<'a> GlPipelineArguments<'a> {
    pub(crate) fn collect_render_targets(
        &self,
        color_targets: &mut smallvec::SmallVec<[&'a GlImage; 8]>,
        depth_stencil_target: &mut Option<&'a GlImage>,
    ) {
        // sub-arguments must be the first block
        let mut blocks = self.blocks.iter();
        if let Some(&StateBlock::Inherited(args)) = blocks.next() {
            for a in args {
                a.collect_render_targets(color_targets, depth_stencil_target);
            }
        }

        while let Some(block) = blocks.next() {
            match block {
                &StateBlock::RenderTarget(rt) => color_targets.extend_from_slice(rt),
                &StateBlock::DepthStencilRenderTarget(rt) => *depth_stencil_target = Some(rt),
                _ => {}
            }
        }
    }

    pub(crate) fn new<'b>(
        arena: &'a GlArena,
        gl: &Gl,
        sampler_cache: &mut SamplerCache,
        signature: &'a GlPipelineSignature,
        arguments: impl IntoIterator<Item = PipelineArgumentsTypeless<'a, OpenGlBackend>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, OpenGlBackend>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferDescriptor<'a, 'b, OpenGlBackend>>,
        index_buffer: Option<IndexBufferDescriptor<'a, OpenGlBackend>>,
        render_targets: impl IntoIterator<Item = RenderTargetDescriptor<'a, OpenGlBackend>>,
        depth_stencil_render_target: Option<RenderTargetDescriptor<'a, OpenGlBackend>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = ScissorRect>,
    ) -> GlPipelineArguments<'a> {
        // TODO This function is a bit hard to digest: refactor

        // must check signature by counting
        // if expected zero, do not allocate, but count to check for size == 0
        // otherwise, alloc as expected, fill, and check expected == count

        let state_blocks = arena
            .other
            .alloc_extend(iter::repeat(StateBlock::Empty).take(signature.num_state_blocks));
        let mut i_block = 0;

        let mut push_state_block = |s: StateBlock<'a>| {
            state_blocks[i_block] = s;
            i_block += 1;
        };

        let args = if signature.sub_signatures.len() > 0 {
            let args = &*arena.other.alloc_extend(arguments.into_iter().map(|a| a.0));
            assert_eq!(signature.sub_signatures.len(), args.len());
            push_state_block(StateBlock::Inherited(args));
            args
        } else {
            assert_eq!(arguments.into_iter().count(), 0);
            &[]
        };

        if signature.num_uniform_buffers > 0
            || signature.num_textures > 0
            || signature.num_images > 0
            || signature.num_shader_storage_buffers > 0
        {
            let uniform_buffers = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_uniform_buffers)
            };
            let uniform_buffer_offsets = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_uniform_buffers)
            };
            let uniform_buffer_sizes = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_uniform_buffers)
            };

            let shader_storage_buffers = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_shader_storage_buffers)
            };
            let shader_storage_buffer_offsets = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_shader_storage_buffers)
            };
            let shader_storage_buffer_sizes = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_shader_storage_buffers)
            };

            let textures: &mut [GLuint] =
                unsafe { arena.other.alloc_uninitialized(signature.num_textures) };
            let images: &mut [GLuint] =
                unsafe { arena.other.alloc_uninitialized(signature.num_images) };
            let samplers: &mut [GLuint] =
                unsafe { arena.other.alloc_uninitialized(signature.num_textures) };

            let mut i_uniform = 0;
            let mut i_storage = 0;
            let mut i_texture = 0;
            let mut i_image = 0;
            for (i, descriptor) in descriptors.into_iter().enumerate() {
                let ty = signature.descriptor_map[i];
                match ty {
                    DescriptorType::SampledImage => match descriptor {
                        Descriptor::SampledImage { img, ref sampler } => {
                            textures[i_texture] = img.raw.obj;
                            samplers[i_texture] = sampler_cache.get_sampler(gl, sampler);
                            i_texture += 1;
                        }
                        _ => panic!("unexpected descriptor type"),
                    },
                    DescriptorType::StorageImage => match descriptor {
                        Descriptor::Image { img } => {
                            images[i_image] = img.raw.obj;
                            i_image += 1;
                        }
                        _ => panic!("unexpected descriptor type"),
                    },
                    DescriptorType::UniformBuffer => match descriptor {
                        Descriptor::Buffer {
                            buffer,
                            offset,
                            size,
                        } => {
                            uniform_buffers[i_uniform] = buffer.raw.obj;
                            uniform_buffer_offsets[i_uniform] = (buffer.offset + offset) as isize;
                            uniform_buffer_sizes[i_uniform] =
                                size.unwrap_or(buffer.raw.size - offset) as isize;
                            i_uniform += 1;
                        }
                        _ => panic!("unexpected descriptor type"),
                    },
                    DescriptorType::StorageBuffer => match descriptor {
                        Descriptor::Buffer {
                            buffer,
                            offset,
                            size,
                        } => {
                            shader_storage_buffers[i_storage] = buffer.raw.obj;
                            shader_storage_buffer_offsets[i_storage] =
                                (buffer.offset + offset) as isize;
                            shader_storage_buffer_sizes[i_storage] =
                                size.unwrap_or(buffer.raw.size - offset) as isize;
                            i_storage += 1;
                        }
                        _ => panic!("unexpected descriptor type"),
                    },
                    DescriptorType::InputAttachment => unimplemented!(),
                    DescriptorType::Sampler => unimplemented!(),
                }
            }

            //
            assert_eq!(signature.num_textures, i_texture);
            assert_eq!(signature.num_images, i_image);
            assert_eq!(signature.num_shader_storage_buffers, i_storage);
            assert_eq!(signature.num_uniform_buffers, i_uniform);

            if signature.num_textures > 0 {
                push_state_block(StateBlock::Textures(textures));
            }
            if signature.num_images > 0 {
                push_state_block(StateBlock::Images(images));
            }
            if signature.num_uniform_buffers > 0 {
                push_state_block(StateBlock::UniformBuffers {
                    buffers: uniform_buffers,
                    offsets: uniform_buffer_offsets,
                    sizes: uniform_buffer_sizes,
                });
            }
            if signature.num_shader_storage_buffers > 0 {
                push_state_block(StateBlock::ShaderStorageBuffers {
                    buffers: shader_storage_buffers,
                    offsets: shader_storage_buffer_offsets,
                    sizes: shader_storage_buffer_sizes,
                });
            }
        } else {
            assert_eq!(descriptors.into_iter().count(), 0);
        }

        if signature.num_vertex_buffers > 0 {
            let vbo = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_vertex_buffers)
            };
            let vb_offsets = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_vertex_buffers)
            };
            let vb_strides = unsafe {
                arena
                    .other
                    .alloc_uninitialized(signature.num_vertex_buffers)
            };

            let mut i = 0;
            for vb in vertex_buffers.into_iter() {
                vbo[i] = vb.buffer.0.raw.obj;
                vb_offsets[i] = (vb.offset as usize + vb.buffer.0.offset) as isize;
                vb_strides[i] = vb.layout.stride as GLsizei;
                i += 1;
            }

            assert_eq!(signature.num_vertex_buffers, i);

            push_state_block(StateBlock::VertexBuffers {
                buffers: vbo,
                offsets: vb_offsets,
                strides: vb_strides,
            });
        } else {
            assert_eq!(vertex_buffers.into_iter().count(), 0);
        }

        if signature.has_index_buffer {
            assert!(index_buffer.is_some());
            // TODO offset, etc.
            let index_buffer = index_buffer.unwrap();
            push_state_block(StateBlock::IndexBuffer {
                buffer: index_buffer.buffer.0.raw.obj,
                offset: index_buffer.buffer.0.offset + index_buffer.offset as usize,
                format: index_buffer.format,
            });
        } else {
            assert!(index_buffer.is_none());
        }

        // create framebuffer if necessary
        if signature.is_root_fragment_output_signature {
            // collect all color attachments
            let mut tmp_color = smallvec::SmallVec::new();
            let mut tmp_depth_stencil = None;
            for a in args {
                a.collect_render_targets(&mut tmp_color, &mut tmp_depth_stencil);
            }
            tmp_color.extend(render_targets.into_iter().map(|rt| rt.image));

            if let Some(dst) = depth_stencil_render_target {
                assert!(tmp_depth_stencil.is_none());
                assert!(signature.has_depth_render_target);
                tmp_depth_stencil = Some(dst.image);
            }

            // build framebuffer
            let fb = GlFramebuffer::new(gl, dbg!(&tmp_color[..]), tmp_depth_stencil)
                .expect("failed to create framebuffer");

            push_state_block(StateBlock::Framebuffer(fb.obj));
        } else {
            if signature.num_render_targets > 0 {
                let rt = arena
                    .other
                    .alloc_extend(render_targets.into_iter().map(|rt| rt.image));
                assert_eq!(rt.len(), signature.num_render_targets);
                push_state_block(StateBlock::RenderTarget(rt));
            } else {
                assert_eq!(render_targets.into_iter().count(), 0);
            }

            if signature.has_depth_render_target {
                assert!(depth_stencil_render_target.is_some());
                push_state_block(StateBlock::DepthStencilRenderTarget(
                    depth_stencil_render_target.unwrap().image,
                ));
            } else {
                assert!(depth_stencil_render_target.is_none());
            }
        }

        let viewports = arena.other.alloc_extend(viewports);
        if viewports.len() > 0 {
            push_state_block(StateBlock::Viewports(viewports));
        }

        let scissors = arena.other.alloc_extend(scissors);
        if scissors.len() > 0 {
            push_state_block(StateBlock::Scissors(scissors));
        }

        GlPipelineArguments {
            signature,
            blocks: state_blocks,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum StateBlock<'a> {
    Inherited(&'a [&'a GlPipelineArguments<'a>]),
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
        strides: &'a [GLsizei],
    },
    IndexBuffer {
        buffer: GLuint,
        format: IndexFormat,
        offset: usize,
    },
    Textures(&'a [GLuint]),
    Images(&'a [GLuint]),
    Samplers(&'a [GLuint]),
    RenderTarget(&'a [&'a GlImage]),
    DepthStencilRenderTarget(&'a GlImage),
    Framebuffer(GLuint),
    Viewports(&'a [Viewport]),
    Scissors(&'a [ScissorRect]),
    Empty,
}
