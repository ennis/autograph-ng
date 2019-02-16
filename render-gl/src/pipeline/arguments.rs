use crate::api::types::*;
use crate::api::Gl;
use crate::backend::GlArena;
use crate::framebuffer::GlFramebuffer;
use crate::image::GlImage;
use crate::sampler::SamplerCache;
use crate::OpenGlBackend;
use autograph_render::descriptor::Descriptor;
use autograph_render::descriptor::DescriptorType;
use autograph_render::framebuffer::RenderTargetDescriptor;
use autograph_render::pipeline::BareArgumentBlock;
use autograph_render::pipeline::ScissorRect;
use autograph_render::pipeline::SignatureDescription;
use autograph_render::pipeline::Viewport;
use autograph_render::vertex::IndexBufferDescriptor;
use autograph_render::vertex::IndexFormat;
use autograph_render::vertex::VertexBufferDescriptor;
use std::iter;
use std::slice;

/// Proposal: flatten signature?
/// At least, no need to store inherited (only the length matters)
#[derive(Clone, Debug)]
pub struct GlSignature {
    //pub(crate) num_inherited: usize,
    pub(crate) inherited: Vec<*const GlSignature>,
    // descriptor #n -> binding space
    pub(crate) descriptor_map: Vec<DescriptorType>,
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

// It's read-only so it should be safe?
unsafe impl Sync for GlSignature {}

impl GlSignature {
    pub(crate) fn new<'a>(
        arena: &'a GlArena,
        inherited: &[&'a GlSignature],
        description: &SignatureDescription,
    ) -> &'a GlSignature {
        // TODO allocate directly in arena when alloc_extend is implemented
        let inherited = inherited
            .iter()
            .map(|&sig| sig as *const _)
            .collect::<Vec<_>>();

        let descriptor_map = description
            .descriptors
            .iter()
            .map(|d| d.descriptor_type)
            .collect::<Vec<_>>();

        // count number of bindings of each type
        //let mut num_vertex_buffers = 0;
        //let mut has_index_buffer = 0;
        let mut num_uniform_buffers = 0;
        let mut num_shader_storage_buffers = 0;
        let mut _num_input_attachments = 0;
        let mut num_textures = 0;
        let mut num_images = 0;
        let mut _num_samplers = 0;
        //let mut num_render_targets = 0;
        for d in descriptor_map.iter() {
            match d {
                DescriptorType::SampledImage => num_textures += 1,
                DescriptorType::StorageImage => num_images += 1,
                DescriptorType::UniformBuffer => num_uniform_buffers += 1,
                DescriptorType::StorageBuffer => num_shader_storage_buffers += 1,
                DescriptorType::InputAttachment => _num_input_attachments += 1,
                DescriptorType::Sampler => _num_samplers += 1,
            }
        }
        let num_vertex_buffers = description.vertex_layouts.len();
        let has_index_buffer = description.index_format.is_some();
        let num_render_targets = description.fragment_outputs.len();
        let has_depth_render_target = description.depth_stencil_fragment_output.is_some();

        let mut num_state_blocks = 0;
        if num_textures > 0 {
            // textures and samplers
            num_state_blocks += 2;
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
        if description.is_root_fragment_output_signature {
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

        arena.signatures.alloc(GlSignature {
            inherited,
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
            is_root_fragment_output_signature: description.is_root_fragment_output_signature,
            is_root_vertex_input_signature: description.is_root_vertex_input_signature,
        })
    }
}

//const MAX_INLINE_ARGUMENTS: usize = 8;

/*pub(crate) struct GlIndexBuffer {
    buffer: GLuint,
    offset: GLintptr,
    fmt: IndexFormat,
}*/

/// All references in fields are raw pointers because this type is an associated type of OpenGlBackend, which cannot be generic, and thus
/// cannot have lifetime parameters (for now, until rust has ATC's)
///
/// Validity of the pointers:
/// - the signature lives at least as long as the arguments (guaranteed by the arena lifetime in the instance API).
/// - the blocks are allocated on the same arena as the object.
///
/// Q: Do we need a ref to the signature?
/// A: Not necessarily
///
/// Q: Could we replace blocks with a safe slice?
/// A: Cannot introduce a lifetime parameter in the struct
/// A2: Actually, can, but must cast to some handle type before returning
#[derive(Copy, Clone, Debug)]
pub struct GlArgumentBlock {
    pub(crate) signature: *const GlSignature,
    pub(crate) blocks: *const StateBlock,
}

unsafe impl Sync for GlArgumentBlock {}

impl GlArgumentBlock {
    /// Unsafe access to contents.
    pub(crate) unsafe fn collect_render_targets<'a>(
        &'a self,
        color_targets: &mut smallvec::SmallVec<[&'a GlImage; 8]>,
        depth_stencil_target: &mut Option<&'a GlImage>,
    ) {
        let signature = &*self.signature;
        // sub-arguments must be the first block
        let blocks = slice::from_raw_parts(self.blocks, signature.num_state_blocks);
        let mut blocks = blocks.iter();
        if let Some(&StateBlock::Inherited(args)) = blocks.next() {
            let args = slice::from_raw_parts(args, signature.inherited.len());
            for &a in args {
                (&*a).collect_render_targets(color_targets, depth_stencil_target);
            }
        }

        while let Some(block) = blocks.next() {
            match block {
                &StateBlock::RenderTarget(rt) => {
                    let num_targets = signature.num_render_targets;
                    let rt = slice::from_raw_parts(rt as *const _, num_targets);
                    color_targets.extend_from_slice(rt)
                }
                &StateBlock::DepthStencilRenderTarget(rt) => *depth_stencil_target = Some(&*rt),
                _ => {}
            }
        }
    }

    pub(crate) fn new<'a, 'b>(
        arena: &'a GlArena,
        gl: &Gl,
        sampler_cache: &mut SamplerCache,
        signature: &'a GlSignature,
        arguments: impl IntoIterator<Item = BareArgumentBlock<'a, OpenGlBackend>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, OpenGlBackend>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferDescriptor<'a, 'b, OpenGlBackend>>,
        index_buffer: Option<IndexBufferDescriptor<'a, OpenGlBackend>>,
        render_targets: impl IntoIterator<Item = RenderTargetDescriptor<'a, OpenGlBackend>>,
        depth_stencil_render_target: Option<RenderTargetDescriptor<'a, OpenGlBackend>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = ScissorRect>,
    ) -> &'a GlArgumentBlock {
        // TODO This function is a bit hard to digest: refactor
        // TODO Gratuitous usage of unsafety here; blame the lack of ATCs

        // must check signature by counting
        // if expected zero, do not allocate, but count to check for size == 0
        // otherwise, alloc as expected, fill, and check expected == count
        let state_blocks = arena
            .other
            .alloc_extend(iter::repeat(StateBlock::Empty).take(signature.num_state_blocks));

        let mut i_block = 0;

        let mut push_state_block = |s: StateBlock| {
            assert!(i_block < signature.num_state_blocks);
            state_blocks[i_block] = s;
            i_block += 1;
        };

        let args = if signature.inherited.len() > 0 {
            let args = &*arena
                .other
                .alloc_extend(arguments.into_iter().map(|a| a.0 as *const _));
            assert_eq!(signature.inherited.len(), args.len());
            push_state_block(StateBlock::Inherited(args.as_ptr()));
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
                push_state_block(StateBlock::Textures(textures.as_ptr()));
                push_state_block(StateBlock::Samplers(samplers.as_ptr()));
            }
            if signature.num_images > 0 {
                push_state_block(StateBlock::Images(images.as_ptr()));
            }
            if signature.num_uniform_buffers > 0 {
                push_state_block(StateBlock::UniformBuffers {
                    buffers: uniform_buffers.as_ptr(),
                    offsets: uniform_buffer_offsets.as_ptr(),
                    sizes: uniform_buffer_sizes.as_ptr(),
                });
            }
            if signature.num_shader_storage_buffers > 0 {
                push_state_block(StateBlock::ShaderStorageBuffers {
                    buffers: shader_storage_buffers.as_ptr(),
                    offsets: shader_storage_buffer_offsets.as_ptr(),
                    sizes: shader_storage_buffer_sizes.as_ptr(),
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
                buffers: vbo.as_ptr(),
                offsets: vb_offsets.as_ptr(),
                strides: vb_strides.as_ptr(),
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
            for &a in args {
                unsafe {
                    (&*a).collect_render_targets(&mut tmp_color, &mut tmp_depth_stencil);
                }
            }
            tmp_color.extend(render_targets.into_iter().map(|rt| rt.image));

            if let Some(dst) = depth_stencil_render_target {
                assert!(tmp_depth_stencil.is_none());
                assert!(signature.has_depth_render_target);
                tmp_depth_stencil = Some(dst.image);
            }

            // build framebuffer
            // put in arena so that it's deleted at the same time as the argument block
            let fb = arena.framebuffers.alloc(
                GlFramebuffer::new(gl, dbg!(&tmp_color[..]), tmp_depth_stencil)
                    .expect("failed to create framebuffer"),
            );

            push_state_block(StateBlock::Framebuffer(fb.obj));
        } else {
            if signature.num_render_targets > 0 {
                let rt = arena
                    .other
                    .alloc_extend(render_targets.into_iter().map(|rt| rt.image as *const _));
                assert_eq!(rt.len(), signature.num_render_targets);
                push_state_block(StateBlock::RenderTarget(rt.as_ptr()));
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
            push_state_block(StateBlock::Viewports(viewports.len(), viewports.as_ptr()));
        }

        let scissors = arena.other.alloc_extend(scissors);
        if scissors.len() > 0 {
            push_state_block(StateBlock::Scissors(scissors.len(), scissors.as_ptr()));
        }

        let args = GlArgumentBlock {
            signature: signature as *const _,
            blocks: state_blocks.as_ptr(),
        };

        arena.other.alloc(args)
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum StateBlock {
    Inherited(*const *const GlArgumentBlock),
    UniformBuffers {
        buffers: *const GLuint,
        offsets: *const GLintptr,
        sizes: *const GLintptr,
    },
    ShaderStorageBuffers {
        buffers: *const GLuint,
        offsets: *const GLintptr,
        sizes: *const GLintptr,
    },
    VertexBuffers {
        buffers: *const GLuint,
        offsets: *const GLintptr,
        strides: *const GLsizei,
    },
    IndexBuffer {
        buffer: GLuint,
        format: IndexFormat,
        offset: usize,
    },
    Textures(*const GLuint),
    Images(*const GLuint),
    Samplers(*const GLuint),
    RenderTarget(*const *const GlImage),
    DepthStencilRenderTarget(*const GlImage),
    Framebuffer(GLuint),
    Viewports(usize, *const Viewport),
    Scissors(usize, *const ScissorRect),
    Empty,
}
