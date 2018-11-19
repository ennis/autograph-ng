use gfx::{
    BufferData, BufferSliceAny, Framebuffer, FramebufferObject, GraphicsPipeline,
    GraphicsShaderPipeline, Sampler, TextureAny,
};
use gl;
use gl::types::*;

const MAX_TEXTURE_UNITS: usize = 16;
const MAX_IMAGE_UNITS: usize = 8;
const MAX_VERTEX_BUFFER_SLOTS: usize = 8;
const MAX_UNIFORM_BUFFER_SLOTS: usize = 8;
const MAX_SHADER_STORAGE_BUFFER_SLOTS: usize = 8;

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub(super) struct Uniforms {
    pub(super) textures: [GLuint; MAX_TEXTURE_UNITS],
    pub(super) samplers: [GLuint; MAX_TEXTURE_UNITS],
    pub(super) images: [GLuint; MAX_IMAGE_UNITS],
    pub(super) uniform_buffers: [GLuint; MAX_UNIFORM_BUFFER_SLOTS],
    pub(super) uniform_buffer_sizes: [GLsizeiptr; MAX_UNIFORM_BUFFER_SLOTS],
    pub(super) uniform_buffer_offsets: [GLintptr; MAX_UNIFORM_BUFFER_SLOTS],
    pub(super) shader_storage_buffers: [GLuint; MAX_SHADER_STORAGE_BUFFER_SLOTS],
    pub(super) shader_storage_buffer_sizes: [GLsizeiptr; MAX_SHADER_STORAGE_BUFFER_SLOTS],
    pub(super) shader_storage_buffer_offsets: [GLintptr; MAX_SHADER_STORAGE_BUFFER_SLOTS],
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub(super) struct VertexInput {
    pub(super) vertex_buffers: [GLuint; MAX_VERTEX_BUFFER_SLOTS],
    pub(super) vertex_buffer_strides: [GLsizei; MAX_VERTEX_BUFFER_SLOTS],
    pub(super) vertex_buffer_offsets: [GLintptr; MAX_VERTEX_BUFFER_SLOTS],
    pub(super) index_buffer: GLuint,
    pub(super) index_buffer_offset: usize,
    pub(super) index_buffer_size: usize,
    pub(super) index_buffer_type: GLenum,
}

// TODO is this useless?
bitflags! {
    #[derive(Default)]
    pub struct StateGroupMask: u32 {
        ///
        const SG_VIEWPORTS = (1 << 0); // DONE
        const SG_FRAMEBUFFER = (1 << 1); // DONE
        const SG_SCISSOR_RECT = (1 << 2);
        const SG_BLEND_STATE = (1 << 3); // DONE
        const SG_RASTERIZER_STATE = (1 << 4); // DONE
        const SG_DEPTH_STENCIL_STATE = (1 << 5); // DONE
        const SG_TEXTURES = (1 << 6); // DONE
        const SG_SAMPLERS = (1 << 7);
        const SG_UNIFORM_BUFFERS = (1 << 8); // DONE
        const SG_SHADER_STORAGE_BUFFERS = (1 << 9); // DONE
        const SG_VERTEX_ARRAY = (1 << 10); // DONE
        const SG_PROGRAM = (1 << 11); // DONE
        const SG_VERTEX_BUFFERS = (1 << 12); // DONE
        const SG_INDEX_BUFFER = (1 << 13); // DONE
        const SG_IMAGE = (1 << 14); // DONE
        const SG_ALL_COMPUTE = SG_IMAGE.bits | SG_TEXTURES.bits | SG_SAMPLERS.bits | SG_PROGRAM.bits | SG_UNIFORM_BUFFERS.bits | SG_SHADER_STORAGE_BUFFERS.bits;
        const SG_ALL = 0xFFFFFFF;
    }
}

pub(super) unsafe fn bind_graphics_pipeline(pipe: &GraphicsPipeline, mask: StateGroupMask) {
    if mask.contains(SG_BLEND_STATE) {
        gl::Enable(gl::BLEND); // XXX is this necessary
        for (i, bs) in pipe.blend_states.iter().enumerate() {
            if bs.enabled {
                gl::Enablei(gl::BLEND, i as u32);
                gl::BlendEquationSeparatei(i as u32, bs.mode_rgb, bs.mode_alpha);
                gl::BlendFuncSeparatei(
                    i as u32,
                    bs.func_src_rgb,
                    bs.func_dst_rgb,
                    bs.func_src_alpha,
                    bs.func_dst_alpha,
                );
            } else {
                gl::Disablei(gl::BLEND, i as u32);
            }
        }
    }

    if mask.contains(SG_DEPTH_STENCIL_STATE) {
        if pipe.depth_stencil_state.depth_test_enable {
            gl::Enable(gl::DEPTH_TEST);
        } else {
            gl::Disable(gl::DEPTH_TEST);
        }

        if pipe.depth_stencil_state.depth_write_enable {
            gl::DepthMask(gl::TRUE);
        } else {
            gl::DepthMask(gl::FALSE);
        }

        gl::DepthFunc(pipe.depth_stencil_state.depth_test_func);

        if pipe.depth_stencil_state.stencil_enable {
            unimplemented!("Stencil buffers")
        } else {
            gl::Disable(gl::STENCIL_TEST);
        }
    }

    if mask.contains(SG_RASTERIZER_STATE) {
        gl::PolygonMode(gl::FRONT_AND_BACK, pipe.rasterizer_state.fill_mode);
        gl::Disable(gl::CULL_FACE);
    }

    if mask.contains(SG_VERTEX_ARRAY) {
        gl::BindVertexArray(pipe.vao);
    }

    if mask.contains(SG_PROGRAM) {
        gl::UseProgram(pipe.shader_pipeline.get_program().unwrap());
    }
}

pub(super) unsafe fn bind_uniforms(uniforms: &Uniforms) {
    // Textures
    gl::BindTextures(0, MAX_TEXTURE_UNITS as i32, uniforms.textures.as_ptr());
    // Samplers
    gl::BindSamplers(0, MAX_TEXTURE_UNITS as i32, uniforms.samplers.as_ptr());
    // Images
    gl::BindImageTextures(0, MAX_IMAGE_UNITS as i32, uniforms.images.as_ptr());

    // UBOs
    for i in 0..MAX_UNIFORM_BUFFER_SLOTS {
        if uniforms.uniform_buffers[i] != 0 {
            gl::BindBufferRange(
                gl::UNIFORM_BUFFER,
                i as u32,
                uniforms.uniform_buffers[i],
                uniforms.uniform_buffer_offsets[i],
                uniforms.uniform_buffer_sizes[i],
            );
        } else {
            gl::BindBufferBase(gl::UNIFORM_BUFFER, i as u32, 0);
        }
    }

    // SSBOs
    for i in 0..MAX_SHADER_STORAGE_BUFFER_SLOTS {
        if uniforms.shader_storage_buffers[i] != 0 {
            gl::BindBufferRange(
                gl::SHADER_STORAGE_BUFFER,
                i as u32,
                uniforms.shader_storage_buffers[i],
                uniforms.shader_storage_buffer_offsets[i],
                uniforms.shader_storage_buffer_sizes[i],
            );
        } else {
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, i as u32, 0);
        }
    }
}

pub(super) unsafe fn bind_vertex_input(vertex_input: &VertexInput) {
    for i in 0..vertex_input.vertex_buffers.len() {
        if vertex_input.vertex_buffers[i] != 0 {
            gl::BindVertexBuffer(
                i as u32,
                vertex_input.vertex_buffers[i],
                vertex_input.vertex_buffer_offsets[i],
                vertex_input.vertex_buffer_strides[i],
            );
        } else {
            gl::BindVertexBuffer(i as u32, 0, 0, 0);
        }
    }

    if vertex_input.index_buffer != 0 {
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, vertex_input.index_buffer);
    }
}

pub enum Scissors {
    All(Option<(i32, i32, i32, i32)>),
}

pub(super) unsafe fn bind_scissors(scissors: &Scissors) {
    match scissors {
        &Scissors::All(None) => gl::Disable(gl::SCISSOR_TEST),
        &Scissors::All(Some((x, y, w, h))) => {
            gl::Enable(gl::SCISSOR_TEST);
            gl::Scissor(x, y, w, h);
        }
    }
}

pub(super) unsafe fn bind_target(framebuffer: &Framebuffer, viewport: &[(f32, f32, f32, f32)]) {
    gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, framebuffer.obj);
    gl::ViewportArrayv(0, 8, viewport.as_ptr() as *const GLfloat);
}

pub struct StateCache {
    /// All uniforms
    //uniforms: Option<Uniforms>,
    //vertex_input: Option<VertexInput>,
    framebuffer: Option<*const FramebufferObject>,
    pipeline: Option<*const super::pipeline::inner::GraphicsPipeline>,
    scissors: Option<Scissors>,
}

impl StateCache {
    pub(super) fn new() -> StateCache {
        StateCache {
            //uniforms: None,
            //vertex_input: None,
            pipeline: None,
            framebuffer: None,
            scissors: None,
        }
    }

    pub unsafe fn set_graphics_pipeline(&mut self, pipe: &GraphicsPipeline) {
        // same pipeline as before?
        if self
            .pipeline
            .map_or(true, |prev_pipe| prev_pipe != pipe.as_ref() as *const _)
        {
            // nope, bind it
            // TODO fine-grained state changes
            bind_graphics_pipeline(pipe, SG_ALL);
            self.pipeline = Some(pipe.as_ref() as *const _);
        }
    }

    pub unsafe fn set_uniform_buffer(&mut self, slot: u32, buffer: &BufferSliceAny) {
        // TODO batch and cache
        gl::BindBufferRange(
            gl::UNIFORM_BUFFER,
            slot,
            buffer.owner.gl_object(),
            buffer.offset as isize,
            buffer.byte_size as isize,
        );
    }

    pub unsafe fn set_shader_storage_buffer(&mut self, slot: u32, buffer: &BufferSliceAny) {
        // TODO batch and cache
        gl::BindBufferRange(
            gl::SHADER_STORAGE_BUFFER,
            slot,
            buffer.owner.gl_object(),
            buffer.offset as isize,
            buffer.byte_size as isize,
        );
    }

    pub unsafe fn set_sampler(&self, index: u32, sampler: GLuint) {
        unimplemented!()
    }

    pub unsafe fn set_vertex_buffer(&self, slot: u32, buffer: &BufferSliceAny, stride: usize) {
        // No caching
        gl::BindVertexBuffer(
            slot,
            buffer.owner.gl_object(),
            buffer.offset as isize,
            stride as i32,
        );
    }

    pub unsafe fn set_index_buffer(&self, buffer: &BufferSliceAny) {
        // TODO cache
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, buffer.owner.gl_object());
    }

    pub unsafe fn set_texture(&self, slot: u32, texture: &TextureAny, sampler: &Sampler) {
        // TODO cache and batch
        gl::BindTextureUnit(slot, texture.gl_object());
        gl::BindSampler(slot, sampler.obj);
    }

    pub unsafe fn set_target(
        &mut self,
        framebuffer: &Framebuffer,
        viewport: &[(f32, f32, f32, f32)],
    ) {
        // same framebuffer as before?
        if self.framebuffer.map_or(true, |prev_framebuffer| {
            prev_framebuffer != framebuffer.as_ref() as *const _
        }) {
            // nope, bind it
            bind_target(framebuffer, viewport);
            self.framebuffer = Some(framebuffer.as_ref() as *const _);
        }
    }

    pub unsafe fn set_scissors(&mut self, scissors: &Scissors) {
        // TODO cache
        bind_scissors(scissors);
    }

    /// Commit all uniforms
    pub unsafe fn commit(&self) {
        // TODO
    }

    pub unsafe fn set_uniform_f32(&self, program: u32, location: u32, v: f32) {
        gl::ProgramUniform1f(program, location as i32, v);
    }

    pub unsafe fn set_uniform_vec2(&self, program: u32, location: u32, v: [f32; 2]) {
        gl::ProgramUniform2f(program, location as i32, v[0], v[1]);
    }

    pub unsafe fn set_uniform_vec3(&self, program: u32, location: u32, v: [f32; 3]) {
        gl::ProgramUniform3f(program, location as i32, v[0], v[1], v[2]);
    }

    pub unsafe fn set_uniform_vec4(&self, program: u32, location: u32, v: [f32; 4]) {
        gl::ProgramUniform4f(program, location as i32, v[0], v[1], v[2], v[3]);
    }

    pub unsafe fn set_uniform_i32(&self, program: u32, location: u32, v: i32) {
        unimplemented!()
    }

    pub unsafe fn set_uniform_ivec2(&self, program: u32, location: u32, v: [i32; 2]) {
        unimplemented!()
    }

    pub unsafe fn set_uniform_ivec3(&self, program: u32, location: u32, v: [i32; 3]) {
        unimplemented!()
    }

    pub unsafe fn set_uniform_ivec4(&self, program: u32, location: u32, v: [i32; 4]) {
        unimplemented!()
    }

    pub unsafe fn set_uniform_mat2(&self, program: u32, location: u32, v: [f32; 2 * 2]) {
        unimplemented!()
    }

    pub unsafe fn set_uniform_mat3(&self, program: u32, location: u32, v: [f32; 3 * 3]) {
        unimplemented!()
    }

    pub unsafe fn set_uniform_mat4(&self, program: u32, location: u32, v: [f32; 4 * 4]) {
        unimplemented!()
    }
}
