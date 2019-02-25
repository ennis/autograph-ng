use crate::{
    api as gl,
    api::{types::*, Gl},
    ImplementationParameters,
};
use autograph_render::{
    pipeline::{
        BlendFactor, BlendOp, ColorBlendAttachmentState, CompareOp, CullModeFlags, PolygonMode,
        PrimitiveTopology, Scissor, StencilOp, StencilOpState, StencilTest, Viewport,
    },
    vertex::IndexFormat,
};
use ordered_float::NotNan;

pub struct ColorBlendCache {
    all: bool,
    states: Vec<Option<ColorBlendAttachmentState>>,
}

pub struct StateCache {
    max_draw_buffers: usize,
    _max_color_attachments: usize,
    max_viewports: usize,

    cull_enable: Option<bool>,
    cull_mode: Option<CullModeFlags>,
    polygon_mode: Option<PolygonMode>,
    //front_face: Option<GLenum>,
    program: Option<GLuint>,
    vertex_array: Option<GLuint>,
    framebuffer: Option<GLuint>,

    stencil_test_enabled: Option<bool>,
    stencil_front: Option<StencilOpState>,
    stencil_back: Option<StencilOpState>,

    depth_test_enabled: Option<bool>,
    depth_write_enabled: Option<bool>,
    depth_compare_op: Option<CompareOp>,
    //depth_bounds_test: Option<DepthBoundTest>,
    blend: Option<ColorBlendCache>,
    viewports: Option<Vec<Viewport>>,
    scissors: Option<Vec<Scissor>>,
    //viewports: Option<(Vec<ViewportEntry>, Vec<DepthRangeEntry>)>,
    index_buffer: Option<GLuint>,
    index_buffer_offset: Option<usize>,
    index_buffer_type: Option<GLenum>,
    /*textures: Option<Vec<GLuint>>,
    samplers: Option<Vec<GLuint>>,
    images: Option<Vec<GLuint>>,
    uniform_buffers: Option<Vec<GLuint>>,
    uniform_buffer_sizes: Option<Vec<GLsizeiptr>>,
    uniform_buffer_offsets: Option<Vec<GLintptr>>,
    shader_storage_buffers: Option<Vec<GLuint>>,
    shader_storage_buffer_sizes: Option<Vec<GLsizeiptr>>,
    shader_storage_buffer_offsets: Option<Vec<GLintptr>>,*/
}

fn topology_to_gl(topo: PrimitiveTopology) -> GLenum {
    match topo {
        PrimitiveTopology::TriangleList => gl::TRIANGLES,
        PrimitiveTopology::LineList => gl::LINES,
        PrimitiveTopology::PointList => gl::POINTS,
    }
}

fn stencil_op_to_gl(op: StencilOp) -> GLenum {
    match op {
        StencilOp::Keep => gl::KEEP,
        StencilOp::Zero => gl::ZERO,
        StencilOp::Replace => gl::REPLACE,
        StencilOp::IncrementAndClamp => gl::INCR,
        StencilOp::DecrementAndClamp => gl::DECR,
        StencilOp::Invert => gl::INVERT,
        StencilOp::IncrementAndWrap => gl::INCR_WRAP,
        StencilOp::DecrementAndWrap => gl::DECR_WRAP,
    }
}

fn compare_op_to_gl(op: CompareOp) -> GLenum {
    match op {
        CompareOp::Never => gl::NEVER,
        CompareOp::Less => gl::LESS,
        CompareOp::Equal => gl::EQUAL,
        CompareOp::LessOrEqual => gl::LEQUAL,
        CompareOp::Greater => gl::GREATER,
        CompareOp::NotEqual => gl::NOTEQUAL,
        CompareOp::GreaterOrEqual => gl::GEQUAL,
        CompareOp::Always => gl::ALWAYS,
    }
}

fn blend_factor_to_gl(f: BlendFactor) -> GLenum {
    match f {
        BlendFactor::Zero => gl::ZERO,
        BlendFactor::One => gl::ONE,
        BlendFactor::SrcColor => gl::SRC_COLOR,
        BlendFactor::OneMinusSrcColor => gl::ONE_MINUS_SRC_COLOR,
        BlendFactor::DstColor => gl::DST_COLOR,
        BlendFactor::OneMinusDstColor => gl::ONE_MINUS_DST_COLOR,
        BlendFactor::SrcAlpha => gl::SRC_ALPHA,
        BlendFactor::OneMinusSrcAlpha => gl::ONE_MINUS_SRC_ALPHA,
        BlendFactor::DstAlpha => gl::DST_ALPHA,
        BlendFactor::OneMinusDstAlpha => gl::ONE_MINUS_DST_ALPHA,
        BlendFactor::ConstantColor => gl::CONSTANT_COLOR,
        BlendFactor::OneMinusConstantColor => gl::ONE_MINUS_CONSTANT_COLOR,
        BlendFactor::ConstantAlpha => gl::CONSTANT_ALPHA,
        BlendFactor::OneMinusConstantAlpha => gl::ONE_MINUS_CONSTANT_ALPHA,
        BlendFactor::SrcAlphaSaturate => gl::SRC_ALPHA_SATURATE,
        BlendFactor::Src1Color => gl::SRC1_COLOR,
        BlendFactor::OneMinusSrc1Color => gl::ONE_MINUS_SRC1_COLOR,
        BlendFactor::Src1Alpha => gl::SRC1_ALPHA,
        BlendFactor::OneMinusSrc1Alpha => gl::ONE_MINUS_SRC1_ALPHA,
    }
}

fn blend_op_to_gl(op: BlendOp) -> GLenum {
    match op {
        BlendOp::Add => gl::FUNC_ADD,
        BlendOp::Subtract => gl::FUNC_SUBTRACT,
        BlendOp::ReverseSubtract => gl::FUNC_REVERSE_SUBTRACT,
        BlendOp::Min => gl::MIN,
        BlendOp::Max => gl::MAX,
    }
}

trait CacheOptionExt<T: Eq> {
    fn update_cached<F: FnOnce()>(&mut self, new: T, f: F);
}

impl<T: Eq> CacheOptionExt<T> for Option<T> {
    fn update_cached<F: FnOnce()>(&mut self, new: T, f: F) {
        if self.as_ref().map_or(true, |v| *v != new) {
            self.replace(new);
            f();
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
struct ViewportEntry {
    // OK to use NotNan because it's repr(transparent) for f32
    x: NotNan<f32>,
    y: NotNan<f32>,
    width: NotNan<f32>,
    height: NotNan<f32>,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct DepthRangeEntry {
    min: f64,
    max: f64,
}

fn update_cached_slice<T: Copy + Eq>(
    cached: &mut Option<Vec<T>>,
    first: usize,
    new: &[T],
    max: usize,
    default: T,
) -> bool {
    let len = new.len();
    assert!(first + len <= max);
    if let Some(cur) = cached.as_mut() {
        let len = new.len();
        if new != &cur[first..len] {
            (&mut cur[first..len]).copy_from_slice(new);
            true
        } else {
            false
        }
    } else {
        let mut v = vec![default; max];
        (&mut v[first..len]).copy_from_slice(new);
        *cached = Some(v);
        true
    }
}

impl StateCache {
    pub fn new(params: &ImplementationParameters) -> StateCache {
        StateCache {
            max_draw_buffers: params.max_draw_buffers as usize,
            _max_color_attachments: params.max_color_attachments as usize,
            max_viewports: params.max_viewports as usize,
            cull_enable: None,
            cull_mode: None,
            polygon_mode: None,
            //front_face: None,
            program: None,
            vertex_array: None,
            framebuffer: None,
            stencil_test_enabled: None,
            stencil_front: None,
            stencil_back: None,
            depth_test_enabled: None,
            depth_write_enabled: None,
            depth_compare_op: None,
            //depth_bounds_test: None,
            blend: None,
            viewports: None,
            scissors: None,
            index_buffer: None,
            index_buffer_offset: None,
            index_buffer_type: None,
            /*textures: None,
            samplers: None,
            images: None,
            uniform_buffers: None,
            uniform_buffer_sizes: None,
            uniform_buffer_offsets: None,
            shader_storage_buffers: None,
            shader_storage_buffer_sizes: None,
            shader_storage_buffer_offsets: None,*/
        }
    }

    pub fn invalidate(&mut self) {
        *self = StateCache {
            max_draw_buffers: self.max_draw_buffers,
            _max_color_attachments: self._max_color_attachments,
            max_viewports: self.max_viewports,
            cull_enable: None,
            cull_mode: None,
            polygon_mode: None,
            //front_face: None,
            program: None,
            vertex_array: None,
            framebuffer: None,
            stencil_test_enabled: None,
            stencil_front: None,
            stencil_back: None,
            depth_test_enabled: None,
            depth_write_enabled: None,
            depth_compare_op: None,
            //depth_bounds_test: None,
            blend: None,
            viewports: None,
            scissors: None,
            index_buffer: None,
            index_buffer_offset: None,
            index_buffer_type: None,
            /*textures: None,
            samplers: None,
            images: None,
            uniform_buffers: None,
            uniform_buffer_sizes: None,
            uniform_buffer_offsets: None,
            shader_storage_buffers: None,
            shader_storage_buffer_sizes: None,
            shader_storage_buffer_offsets: None,*/
        };
    }

    pub fn set_program(&mut self, gl: &Gl, program: GLuint) {
        self.program.update_cached(program, || unsafe {
            gl.UseProgram(program);
        });
    }

    pub fn set_vertex_array(&mut self, gl: &Gl, vertex_array: GLuint) {
        self.vertex_array.update_cached(vertex_array, || unsafe {
            gl.BindVertexArray(vertex_array);
        });
    }

    pub fn set_draw_framebuffer(&mut self, gl: &Gl, framebuffer: GLuint) {
        self.framebuffer.update_cached(framebuffer, || unsafe {
            gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, framebuffer);
        });
    }

    pub fn set_all_blend(&mut self, gl: &Gl, state: &ColorBlendAttachmentState) {
        let bind_all = |state: &ColorBlendAttachmentState| match state {
            ColorBlendAttachmentState::Disabled => unsafe { gl.Disable(gl::BLEND) },
            ColorBlendAttachmentState::Enabled {
                src_color_blend_factor,
                dst_color_blend_factor,
                color_blend_op,
                src_alpha_blend_factor,
                dst_alpha_blend_factor,
                alpha_blend_op,
                color_write_mask: _,
            } => unsafe {
                gl.Enable(gl::BLEND);
                gl.BlendEquationSeparate(
                    blend_op_to_gl(*color_blend_op),
                    blend_op_to_gl(*alpha_blend_op),
                );
                gl.BlendFuncSeparate(
                    blend_factor_to_gl(*src_color_blend_factor),
                    blend_factor_to_gl(*dst_color_blend_factor),
                    blend_factor_to_gl(*src_alpha_blend_factor),
                    blend_factor_to_gl(*dst_alpha_blend_factor),
                );
            },
        };

        if let Some(ref mut blend) = self.blend {
            if !(blend.all == true && blend.states[0].as_ref() == Some(state)) {
                blend.all = true;
                for s in blend.states.iter_mut() {
                    *s = Some(*state);
                }
                // enable all at once
                bind_all(state);
            }
        } else {
            self.blend = Some(ColorBlendCache {
                all: true,
                states: vec![Some(*state); self.max_draw_buffers as usize],
            });
            bind_all(state);
        }
    }

    pub fn set_blend_separate(&mut self, gl: &Gl, index: u32, state: &ColorBlendAttachmentState) {
        let bind_separate = |index: u32, state: &ColorBlendAttachmentState| match state {
            ColorBlendAttachmentState::Disabled => unsafe { gl.Disablei(gl::BLEND, index) },
            ColorBlendAttachmentState::Enabled {
                src_color_blend_factor,
                dst_color_blend_factor,
                color_blend_op,
                src_alpha_blend_factor,
                dst_alpha_blend_factor,
                alpha_blend_op,
                color_write_mask: _,
            } => unsafe {
                gl.Enablei(gl::BLEND, index);
                gl.BlendEquationSeparatei(
                    index,
                    blend_op_to_gl(*color_blend_op),
                    blend_op_to_gl(*alpha_blend_op),
                );
                gl.BlendFuncSeparatei(
                    index,
                    blend_factor_to_gl(*src_color_blend_factor),
                    blend_factor_to_gl(*dst_color_blend_factor),
                    blend_factor_to_gl(*src_alpha_blend_factor),
                    blend_factor_to_gl(*dst_alpha_blend_factor),
                );
            },
        };

        if let Some(ref mut blend) = self.blend {
            if blend.states[index as usize].as_ref() != Some(state) {
                blend.all = false;
                blend.states[index as usize] = Some(*state);
                bind_separate(index, state);
            }
        } else {
            let mut states = vec![None; self.max_draw_buffers as usize];
            states[index as usize] = Some(*state);
            self.blend = Some(ColorBlendCache { all: false, states });
            bind_separate(index, state);
        }
    }

    pub fn set_viewports(&mut self, gl: &Gl, viewports: &[Viewport]) {
        let do_update = update_cached_slice(
            &mut self.viewports,
            0,
            viewports,
            self.max_viewports as usize,
            Viewport {
                x: 0.0.into(),
                y: 0.0.into(),
                width: 0.0.into(),
                height: 0.0.into(),
                min_depth: 0.0.into(),
                max_depth: 0.0.into(),
            },
        );

        if do_update {
            let mut gl_viewports: Vec<ViewportEntry> = Vec::with_capacity(self.max_viewports);
            let mut gl_depth_ranges: Vec<DepthRangeEntry> = Vec::with_capacity(self.max_viewports);

            for vp in self
                .viewports
                .as_ref()
                .unwrap()
                .iter()
                .take(viewports.len())
            {
                gl_viewports.push(ViewportEntry {
                    x: vp.x,
                    y: vp.y,
                    width: vp.width,
                    height: vp.height,
                });
                gl_depth_ranges.push(DepthRangeEntry {
                    min: vp.min_depth.into_inner() as f64,
                    max: vp.max_depth.into_inner() as f64,
                });
            }

            unsafe {
                gl.ViewportArrayv(0, viewports.len() as i32, gl_viewports.as_ptr() as *const _);
                gl.DepthRangeArrayv(
                    0,
                    viewports.len() as i32,
                    gl_depth_ranges.as_ptr() as *const f64,
                );
            }
        }
    }

    pub fn set_scissors(&mut self, gl: &Gl, scissors: &[Scissor]) {
        let do_update = update_cached_slice(
            &mut self.scissors,
            0,
            scissors,
            self.max_viewports,
            Scissor::Disabled,
        );

        if do_update {
            #[repr(C)]
            #[derive(Copy, Clone)]
            struct GlScissor {
                x: i32,
                y: i32,
                width: i32,
                height: i32,
            }

            let mut gl_scissors: Vec<GlScissor> = Vec::with_capacity(self.max_viewports);

            let mut all_disabled = true;
            let mut all_enabled = true;

            for s in self.scissors.as_ref().unwrap().iter().take(scissors.len()) {
                let s = match s {
                    Scissor::Disabled => {
                        all_enabled = false;
                        GlScissor {
                            x: 0,
                            y: 0,
                            width: 0,
                            height: 0,
                        }
                    }
                    Scissor::Enabled(rect) => {
                        all_disabled = false;
                        GlScissor {
                            x: rect.x,
                            y: rect.y,
                            width: rect.width as i32,
                            height: rect.height as i32,
                        }
                    }
                };
                gl_scissors.push(s);
            }

            unsafe {
                match (all_disabled, all_enabled) {
                    (true, false) => {
                        gl.Disable(gl::SCISSOR_TEST);
                    }
                    (false, true) => {
                        gl.Enable(gl::SCISSOR_TEST);
                        gl.ScissorArrayv(0, scissors.len() as i32, gl_scissors.as_ptr() as *const _)
                    }
                    (false, false) => {
                        // mixed
                        for (i, s) in self
                            .scissors
                            .as_ref()
                            .unwrap()
                            .iter()
                            .take(scissors.len())
                            .enumerate()
                        {
                            match s {
                                Scissor::Disabled => {
                                    gl.Disablei(gl::SCISSOR_TEST, i as u32);
                                }
                                Scissor::Enabled(_) => {
                                    gl.Enablei(gl::SCISSOR_TEST, i as u32);
                                    gl.ScissorIndexedv(
                                        i as u32,
                                        gl_scissors.as_ptr().add(i) as *const _,
                                    );
                                }
                            }
                        }
                    }
                    (true, true) => unreachable!(),
                }
            }
        }
    }

    pub fn set_depth_test_enable(&mut self, gl: &Gl, depth_test_enable: bool) {
        self.depth_test_enabled
            .update_cached(depth_test_enable, || unsafe {
                if depth_test_enable {
                    gl.Enable(gl::DEPTH_TEST);
                } else {
                    gl.Disable(gl::DEPTH_TEST);
                }
            })
    }

    pub fn set_depth_write_enable(&mut self, gl: &Gl, depth_write_enable: bool) {
        self.depth_write_enabled
            .update_cached(depth_write_enable, || unsafe {
                if depth_write_enable {
                    gl.DepthMask(gl::TRUE);
                } else {
                    gl.DepthMask(gl::FALSE);
                }
            })
    }

    //pub fn set_depth_bounds_test(&mut self, )

    pub fn set_depth_compare_op(&mut self, gl: &Gl, depth_compare_op: CompareOp) {
        self.depth_compare_op
            .update_cached(depth_compare_op, || unsafe {
                gl.DepthFunc(compare_op_to_gl(depth_compare_op));
            })
    }

    fn set_cull_enable(&mut self, gl: &Gl, cull_enable: bool) {
        self.cull_enable.update_cached(cull_enable, || unsafe {
            if cull_enable {
                gl.Enable(gl::CULL_FACE);
            } else {
                gl.Disable(gl::CULL_FACE);
            }
        });
    }

    pub fn set_cull_mode(&mut self, gl: &Gl, cull_mode: CullModeFlags) {
        if cull_mode == CullModeFlags::NONE {
            self.set_cull_enable(gl, false);
        } else {
            self.set_cull_enable(gl, true);
        }

        self.cull_mode.update_cached(cull_mode, || unsafe {
            if cull_mode.contains(CullModeFlags::FRONT_AND_BACK) {
                gl.CullFace(gl::FRONT_AND_BACK);
            } else if cull_mode.contains(CullModeFlags::FRONT) {
                gl.CullFace(gl::FRONT);
            } else if cull_mode.contains(CullModeFlags::BACK) {
                gl.CullFace(gl::BACK);
            }
        });
    }

    pub fn set_polygon_mode(&mut self, gl: &Gl, polygon_mode: PolygonMode) {
        self.polygon_mode.update_cached(polygon_mode, || unsafe {
            match polygon_mode {
                PolygonMode::Fill => gl.PolygonMode(gl::FRONT_AND_BACK, gl::FILL),
                PolygonMode::Line => gl.PolygonMode(gl::FRONT_AND_BACK, gl::LINE),
            }
        });
    }

    pub fn set_stencil_test_enabled(&mut self, gl: &Gl, enabled: bool) {
        self.stencil_test_enabled.update_cached(enabled, || unsafe {
            if enabled {
                gl.Enable(gl::STENCIL_TEST);
            } else {
                gl.Disable(gl::STENCIL_TEST);
            }
        });
    }

    /// Does not implicitly enable stencil test.
    pub fn set_stencil_op(&mut self, gl: &Gl, front: &StencilOpState, back: &StencilOpState) {
        let bind_stencil = |face: GLenum, state: &StencilOpState| unsafe {
            gl.StencilFuncSeparate(
                face,
                compare_op_to_gl(state.compare_op),
                state.reference as i32,
                state.compare_mask,
            );
            gl.StencilOpSeparate(
                face,
                stencil_op_to_gl(state.fail_op),
                stencil_op_to_gl(state.depth_fail_op),
                stencil_op_to_gl(state.pass_op),
            );
            gl.StencilMaskSeparate(face, state.write_mask);
        };

        self.stencil_front.update_cached(*front, || {
            bind_stencil(gl::FRONT, front);
        });
        self.stencil_back.update_cached(*back, || {
            bind_stencil(gl::BACK, back);
        });
    }

    pub fn set_stencil_test(&mut self, gl: &Gl, stencil_test: &StencilTest) {
        match stencil_test {
            StencilTest::Disabled => self.set_stencil_test_enabled(gl, false),
            StencilTest::Enabled { front, back } => {
                self.set_stencil_test_enabled(gl, true);
                self.set_stencil_op(gl, front, back);
            }
        }
    }

    pub fn set_uniform_buffers(
        &mut self,
        gl: &Gl,
        first: usize,
        buffers: &[GLuint],
        buffer_offsets: &[GLintptr],
        buffer_sizes: &[GLintptr],
    ) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe {
            let count = buffers.len();
            if count != 0 {
                gl.BindBuffersRange(
                    gl::UNIFORM_BUFFER,
                    first as u32,
                    count as i32,
                    buffers.as_ptr(),
                    buffer_offsets.as_ptr(),
                    buffer_sizes.as_ptr(),
                );
            }
        }
    }

    pub fn set_shader_storage_buffers(
        &mut self,
        gl: &Gl,
        first: usize,
        buffers: &[GLuint],
        buffer_offsets: &[GLintptr],
        buffer_sizes: &[GLintptr],
    ) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe {
            let count = buffers.len();
            if count != 0 {
                gl.BindBuffersRange(
                    gl::SHADER_STORAGE_BUFFER,
                    first as u32,
                    count as i32,
                    buffers.as_ptr(),
                    buffer_offsets.as_ptr(),
                    buffer_sizes.as_ptr(),
                );
            }
        }
    }

    pub fn set_samplers(&mut self, gl: &Gl, first: usize, samplers: &[GLuint]) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe { gl.BindSamplers(first as u32, samplers.len() as i32, samplers.as_ptr()) }
    }

    pub fn set_textures(&mut self, gl: &Gl, first: usize, textures: &[GLuint]) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe { gl.BindTextures(first as u32, textures.len() as i32, textures.as_ptr()) }
    }

    pub fn set_images(&mut self, gl: &Gl, first: usize, images: &[GLuint]) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe { gl.BindImageTextures(first as u32, images.len() as i32, images.as_ptr()) }
    }

    pub fn set_vertex_buffers(
        &mut self,
        gl: &Gl,
        first: usize,
        buffers: &[GLuint],
        buffer_offsets: &[GLintptr],
        buffer_strides: &[GLsizei],
    ) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe {
            let count = buffers.len();
            if count != 0 {
                gl.BindVertexBuffers(
                    first as u32,
                    count as i32,
                    buffers.as_ptr(),
                    buffer_offsets.as_ptr(),
                    buffer_strides.as_ptr(),
                )
            }
        }
    }

    pub fn set_index_buffer(&mut self, gl: &Gl, buffer: GLuint, offset: usize, ty: IndexFormat) {
        self.index_buffer.update_cached(buffer, || unsafe {
            gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, buffer);
        });

        self.index_buffer_offset = Some(offset);
        self.index_buffer_type = Some(match ty {
            IndexFormat::U16 => gl::UNSIGNED_SHORT,
            IndexFormat::U32 => gl::UNSIGNED_INT,
        });
    }

    pub fn draw(
        &mut self,
        gl: &Gl,
        topo: PrimitiveTopology,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        let mode = topology_to_gl(topo);
        unsafe {
            gl.DrawArraysInstancedBaseInstance(
                mode,
                first_vertex as i32,
                vertex_count as i32,
                instance_count as i32,
                first_instance,
            );
        }
    }

    pub fn draw_indexed(
        &mut self,
        gl: &Gl,
        topo: PrimitiveTopology,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        let mode = topology_to_gl(topo);
        let idx_offset = self
            .index_buffer_offset
            .expect("no index buffer was bound before indexed draw operation");
        let ty = self.index_buffer_type.unwrap();
        let idx_stride = match ty {
            gl::UNSIGNED_SHORT => 2,
            gl::UNSIGNED_INT => 4,
            _ => unreachable!(),
        };
        unsafe {
            gl.DrawElementsInstancedBaseVertexBaseInstance(
                mode,
                index_count as i32,
                ty,
                (idx_offset + first_index as usize * idx_stride) as *const GLvoid,
                instance_count as i32,
                vertex_offset,
                first_instance,
            );
        }
    }

    //pub fn set_blend_mode(&mut self)
}
