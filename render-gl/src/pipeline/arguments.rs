use crate::{
    api::{types::*, Gl},
    backend::GlArena,
    framebuffer::GlFramebuffer,
    image::GlImage,
    sampler::SamplerCache,
    OpenGlBackend,
};
use autograph_render::{
    descriptor::{Descriptor, ResourceBindingType},
    image::{DepthStencilView, RenderTargetView},
    pipeline::{BareArgumentBlock, Scissor, SignatureDescription, Viewport},
    vertex::{IndexBufferView, IndexFormat, VertexBufferView},
};
use std::{iter, slice};

/// Proposal: flatten signature?
/// At least, no need to store inherited (only the length matters)
#[derive(Clone, Debug)]
pub struct GlSignature {
    //pub(crate) num_inherited: usize,
    pub(crate) inherited: Vec<*const GlSignature>,
    // descriptor #n -> binding space
    pub(crate) descriptor_map: Vec<ResourceBindingType>,
    pub(crate) num_state_blocks: usize,
    pub(crate) num_vertex_buffers: usize,
    pub(crate) num_uniform_buffers: usize,
    pub(crate) num_shader_storage_buffers: usize,
    pub(crate) num_textures: usize,
    pub(crate) num_images: usize,
    pub(crate) num_viewports: usize,
    pub(crate) num_scissors: usize,
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
            .map(|d| d.ty)
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
                ResourceBindingType::TextureSampler(_) => num_textures += 1,
                ResourceBindingType::RwImage(_) => num_images += 1,
                ResourceBindingType::ConstantBuffer => num_uniform_buffers += 1,
                ResourceBindingType::RwBuffer => num_shader_storage_buffers += 1,
                //ResourceBindingType::InputAttachment => _num_input_attachments += 1,
                ResourceBindingType::TexelBuffer
                | ResourceBindingType::RwTexelBuffer
                | ResourceBindingType::Texture(_)
                | ResourceBindingType::Sampler => unimplemented!(),
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
        if description.num_viewports > 0 {
            num_state_blocks += 1;
        }
        if description.num_scissors > 0 {
            num_state_blocks += 1;
        }
        if inherited.len() > 0 {
            num_state_blocks += 1;
        }

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
            num_viewports: description.num_viewports,
            num_scissors: description.num_scissors,
            is_root_fragment_output_signature: description.is_root_fragment_output_signature,
            is_root_vertex_input_signature: description.is_root_vertex_input_signature,
        })
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
    Viewports(*const Viewport),
    Scissors(*const Scissor),
    Empty,
}

struct StateBlocks<'a> {
    inherited: &'a mut [&'a GlArgumentBlock],
    uniform_buffers: &'a mut [GLuint],
    uniform_buffer_offsets: &'a mut [GLintptr],
    uniform_buffer_sizes: &'a mut [GLintptr],
    shader_storage_buffers: &'a mut [GLuint],
    shader_storage_buffer_offsets: &'a mut [GLintptr],
    shader_storage_buffer_sizes: &'a mut [GLintptr],
    vertex_buffers: &'a mut [GLuint],
    vertex_buffer_offsets: &'a mut [GLintptr],
    vertex_buffer_strides: &'a mut [GLsizei],
    index_buffer: GLuint,
    index_format: IndexFormat,
    index_offset: usize,
    render_targets: &'a mut [&'a GlImage],
    depth_stencil_target: Option<&'a GlImage>,
    textures: &'a mut [GLuint],
    samplers: &'a mut [GLuint],
    images: &'a mut [GLuint],
    framebuffer: GLuint,
    viewports: &'a mut [Viewport],
    scissors: &'a mut [Scissor],
}

impl<'a> StateBlocks<'a> {
    unsafe fn new(arena: &'a GlArena, sig: &GlSignature) -> StateBlocks<'a> {
        let inherited = if sig.inherited.len() != 0 {
            arena.other.alloc_uninitialized(sig.inherited.len())
        } else {
            &mut [][..]
        };

        let (uniform_buffers, uniform_buffer_offsets, uniform_buffer_sizes) =
            if sig.num_uniform_buffers != 0 {
                (
                    arena.other.alloc_uninitialized(sig.num_uniform_buffers),
                    arena.other.alloc_uninitialized(sig.num_uniform_buffers),
                    arena.other.alloc_uninitialized(sig.num_uniform_buffers),
                )
            } else {
                (&mut [][..], &mut [][..], &mut [][..])
            };

        let (shader_storage_buffers, shader_storage_buffer_offsets, shader_storage_buffer_sizes) =
            if sig.num_shader_storage_buffers != 0 {
                (
                    arena
                        .other
                        .alloc_uninitialized(sig.num_shader_storage_buffers),
                    arena
                        .other
                        .alloc_uninitialized(sig.num_shader_storage_buffers),
                    arena
                        .other
                        .alloc_uninitialized(sig.num_shader_storage_buffers),
                )
            } else {
                (&mut [][..], &mut [][..], &mut [][..])
            };

        let (vertex_buffers, vertex_buffer_offsets, vertex_buffer_strides) =
            if sig.num_vertex_buffers != 0 {
                (
                    arena.other.alloc_uninitialized(sig.num_vertex_buffers),
                    arena.other.alloc_uninitialized(sig.num_vertex_buffers),
                    arena.other.alloc_uninitialized(sig.num_vertex_buffers),
                )
            } else {
                (&mut [][..], &mut [][..], &mut [][..])
            };

        let render_targets = if sig.num_render_targets != 0 {
            arena.other.alloc_uninitialized(sig.num_render_targets)
        } else {
            &mut [][..]
        };

        let (textures, samplers) = if sig.num_textures != 0 {
            (
                arena.other.alloc_uninitialized(sig.num_textures),
                arena.other.alloc_uninitialized(sig.num_textures),
            )
        } else {
            (&mut [][..], &mut [][..])
        };

        let images = if sig.num_images != 0 {
            arena.other.alloc_uninitialized(sig.num_images)
        } else {
            &mut [][..]
        };

        let viewports = if sig.num_viewports != 0 {
            arena.other.alloc_uninitialized(sig.num_viewports)
        } else {
            &mut [][..]
        };

        let scissors = if sig.num_scissors != 0 {
            arena.other.alloc_uninitialized(sig.num_scissors)
        } else {
            &mut [][..]
        };

        StateBlocks {
            inherited,
            uniform_buffers,
            uniform_buffer_offsets,
            uniform_buffer_sizes,
            shader_storage_buffers,
            shader_storage_buffer_offsets,
            shader_storage_buffer_sizes,
            vertex_buffers,
            vertex_buffer_offsets,
            vertex_buffer_strides,
            render_targets,
            depth_stencil_target: None,
            index_buffer: 0,
            index_format: IndexFormat::U16,
            index_offset: 0,
            textures,
            images,
            samplers,
            framebuffer: 0,
            viewports,
            scissors,
        }
    }

    unsafe fn into_argument_block(
        self,
        arena: &'a GlArena,
        gl: &Gl,
        signature: &GlSignature,
    ) -> &'a GlArgumentBlock {
        let state_blocks = arena.other.alloc_uninitialized(signature.num_state_blocks);

        let mut i = 0;
        if signature.inherited.len() > 0 {
            state_blocks[i] =
                StateBlock::Inherited(self.inherited.as_ptr() as *const *const GlArgumentBlock);
            i += 1;
        }
        if signature.num_uniform_buffers > 0 {
            state_blocks[i] = StateBlock::UniformBuffers {
                buffers: self.uniform_buffers.as_ptr(),
                offsets: self.uniform_buffer_offsets.as_ptr(),
                sizes: self.uniform_buffer_sizes.as_ptr(),
            };
            i += 1;
        }
        if signature.num_shader_storage_buffers > 0 {
            state_blocks[i] = StateBlock::ShaderStorageBuffers {
                buffers: self.shader_storage_buffers.as_ptr(),
                offsets: self.shader_storage_buffer_offsets.as_ptr(),
                sizes: self.shader_storage_buffer_sizes.as_ptr(),
            };
            i += 1;
        }
        if signature.num_vertex_buffers > 0 {
            state_blocks[i] = StateBlock::VertexBuffers {
                buffers: self.vertex_buffers.as_ptr(),
                offsets: self.vertex_buffer_offsets.as_ptr(),
                strides: self.vertex_buffer_strides.as_ptr(),
            };
            i += 1;
        }
        if signature.num_render_targets > 0 || signature.has_depth_render_target {
            if signature.is_root_fragment_output_signature {
                // collect all color attachments
                let mut tmp_color = smallvec::SmallVec::new();
                let mut tmp_depth_stencil = None;
                // TODO change this when the additional rule that "all render targets must be in the same argument block"
                // is put into place.
                for &a in self.inherited.iter() {
                    unsafe {
                        (&*a).collect_render_targets(&mut tmp_color, &mut tmp_depth_stencil);
                    }
                }
                tmp_color.extend(self.render_targets.iter().cloned());

                if let Some(ds) = self.depth_stencil_target {
                    assert!(tmp_depth_stencil.is_none());
                    assert!(signature.has_depth_render_target);
                    tmp_depth_stencil = Some(ds);
                }

                // build framebuffer
                // put in arena so that it's deleted at the same time as the argument block
                let fb = arena.framebuffers.alloc(
                    GlFramebuffer::new(gl, &tmp_color[..], tmp_depth_stencil)
                        .expect("failed to create framebuffer"),
                );

                state_blocks[i] = StateBlock::Framebuffer(fb.obj);
                i += 1;
            } else {
                // TODO once the new constraint is in place, remove this
                if signature.has_depth_render_target {
                    state_blocks[i] =
                        StateBlock::DepthStencilRenderTarget(self.depth_stencil_target.unwrap());
                    i += 1;
                }
                state_blocks[i] =
                    StateBlock::RenderTarget(self.render_targets.as_ptr() as *const *const GlImage);
                i += 1;
            }
        }
        if signature.num_textures > 0 {
            state_blocks[i] = StateBlock::Textures(self.textures.as_ptr());
            i += 1;
            state_blocks[i] = StateBlock::Samplers(self.samplers.as_ptr());
            i += 1;
        }
        if signature.num_images > 0 {
            state_blocks[i] = StateBlock::Images(self.images.as_ptr());
            i += 1;
        }
        if signature.num_viewports > 0 {
            state_blocks[i] = StateBlock::Viewports(self.viewports.as_ptr());
            i += 1;
        }
        if signature.num_scissors > 0 {
            state_blocks[i] = StateBlock::Scissors(self.scissors.as_ptr());
            i += 1;
        }
        if signature.has_index_buffer {
            state_blocks[i] = StateBlock::IndexBuffer {
                buffer: self.index_buffer,
                format: self.index_format,
                offset: self.index_offset,
            };
            i += 1;
        }

        arena.other.alloc(GlArgumentBlock {
            signature: signature as *const GlSignature,
            blocks: state_blocks.as_ptr(),
        })
    }
}

/// All references in fields are raw pointers because this type is an associated type of OpenGlBackend, which cannot be generic, and thus
/// cannot have lifetime parameters (for now, until rust has ATC's)
///
/// Validity of the pointers:
/// - the signature lives at least as long as the arguments (guaranteed by the arena lifetime in the instance API).
/// - the blocks are allocated on the same arena as the object.
#[derive(Copy, Clone, Debug)]
pub struct GlArgumentBlock {
    pub(crate) signature: *const GlSignature,
    pub(crate) blocks: *const StateBlock,
}

unsafe impl Sync for GlArgumentBlock {}

/// Copy the contents of an iterator into a mut slice
fn copy_iter<T>(mut it: impl Iterator<Item = T>, out: &mut [T]) -> usize {
    let mut i = 0;
    for v in it {
        out[i] = v;
        i += 1;
    }
    i
}

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

    pub(crate) fn new<'a>(
        arena: &'a GlArena,
        gl: &Gl,
        sampler_cache: &mut SamplerCache,
        signature: &'a GlSignature,
        inherited: impl IntoIterator<Item = BareArgumentBlock<'a, OpenGlBackend>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, OpenGlBackend>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferView<'a, OpenGlBackend>>,
        index_buffer: Option<IndexBufferView<'a, OpenGlBackend>>,
        render_targets: impl IntoIterator<Item = RenderTargetView<'a, OpenGlBackend>>,
        depth_stencil_target: Option<DepthStencilView<'a, OpenGlBackend>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = Scissor>,
    ) -> &'a GlArgumentBlock {
        let mut stb = unsafe { StateBlocks::new(arena, signature) };

        let mut i_inherited = copy_iter(inherited.into_iter().map(|a| a.0), stb.inherited);
        let mut i_uniform_buffers: usize = 0;
        let mut i_shader_storage_buffers: usize = 0;
        let mut i_textures_samplers: usize = 0;
        let mut i_images: usize = 0;

        for d in descriptors.into_iter() {
            match d {
                Descriptor::Sampler { desc } => unimplemented!(),
                Descriptor::Texture { image, subresource } => unimplemented!(),
                Descriptor::TextureSampler {
                    image,
                    subresource,
                    sampler,
                } => {
                    if subresource.base_array_layer != 0 || subresource.base_mip_level != 0 {
                        unimplemented!("texture subresource views");
                    }
                    stb.textures[i_textures_samplers] = image.raw.obj;
                    stb.samplers[i_textures_samplers] = sampler_cache.get_sampler(gl, &sampler);
                    i_textures_samplers += 1;
                }
                Descriptor::RwImage { image, subresource } => {
                    if subresource.base_array_layer != 0 || subresource.base_mip_level != 0 {
                        unimplemented!("texture subresource views");
                    }
                    stb.images[i_images] = image.raw.obj;
                    i_images += 1;
                }
                Descriptor::ConstantBuffer {
                    buffer,
                    offset,
                    size,
                } => {
                    stb.uniform_buffers[i_uniform_buffers] = buffer.raw.obj;
                    stb.uniform_buffer_offsets[i_uniform_buffers] =
                        (buffer.offset + offset) as isize;
                    stb.uniform_buffer_sizes[i_uniform_buffers] =
                        size.unwrap_or(buffer.raw.size - offset) as isize;
                    i_uniform_buffers += 1;
                }
                Descriptor::RwBuffer {
                    buffer,
                    offset,
                    size,
                } => {
                    stb.shader_storage_buffers[i_shader_storage_buffers] = buffer.raw.obj;
                    stb.shader_storage_buffer_offsets[i_shader_storage_buffers] =
                        (buffer.offset + offset) as isize;
                    stb.shader_storage_buffer_sizes[i_shader_storage_buffers] =
                        size.unwrap_or(buffer.raw.size - offset) as isize;
                    i_shader_storage_buffers += 1;
                }
                Descriptor::TexelBuffer {
                    buffer,
                    offset,
                    size,
                } => unimplemented!(),
                Descriptor::RwTexelBuffer {
                    buffer,
                    offset,
                    size,
                } => unimplemented!(),
                Descriptor::Empty => unimplemented!(),
            }
        }

        let mut i_vertex_buffers = 0;
        for v in vertex_buffers.into_iter() {
            stb.vertex_buffers[i_vertex_buffers] = v.buffer().raw.obj;
            stb.vertex_buffer_offsets[i_vertex_buffers] = (v.offset() + v.buffer().offset) as isize;
            stb.vertex_buffer_strides[i_vertex_buffers] = v.stride() as i32;
            i_vertex_buffers += 1;
        }

        let i_render_targets = copy_iter(
            render_targets.into_iter().map(|rt| rt.inner()),
            stb.render_targets,
        );

        if let Some(ds) = depth_stencil_target {
            stb.depth_stencil_target = Some(ds.inner());
        }

        if let Some(ib) = index_buffer {
            stb.index_buffer = ib.buffer.raw.obj;
            stb.index_format = ib.format;
            stb.index_offset = ib.buffer.offset + ib.offset;
        }

        let i_viewports = copy_iter(viewports.into_iter(), stb.viewports);
        let i_scissors = copy_iter(scissors.into_iter(), stb.scissors);

        assert_eq!(i_inherited, signature.inherited.len());
        assert_eq!(i_uniform_buffers, signature.num_uniform_buffers);
        assert_eq!(
            i_shader_storage_buffers,
            signature.num_shader_storage_buffers
        );
        assert_eq!(i_vertex_buffers, signature.num_vertex_buffers);
        assert_eq!(i_render_targets, signature.num_render_targets);
        assert_eq!(i_textures_samplers, signature.num_textures);
        assert_eq!(i_images, signature.num_images);
        assert_eq!(i_viewports, signature.num_viewports);
        assert_eq!(i_scissors, signature.num_scissors);

        unsafe { stb.into_argument_block(arena, gl, signature) }
    }
}
