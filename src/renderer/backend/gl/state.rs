use crate::renderer::backend::gl::api as gl;
use crate::renderer::backend::gl::api::types::*;
use crate::renderer::backend::gl::{GraphicsPipeline, ImplementationParameters};
use crate::renderer::*;
use ordered_float::NotNan;

pub struct ViewportState {
    all: bool,
    viewports: Vec<Option<Viewport>>,
}

pub struct ColorBlendCache {
    all: bool,
    states: Vec<Option<PipelineColorBlendAttachmentState>>,
}

pub struct StateCache {
    max_draw_buffers: u32,
    max_color_attachments: u32,
    max_viewports: u32,

    cull_enable: Option<bool>,
    cull_mode: Option<CullModeFlags>,
    polygon_mode: Option<PolygonMode>,
    front_face: Option<GLenum>,
    program: Option<GLuint>,
    vertex_array: Option<GLuint>,

    stencil_test_enabled: Option<bool>,
    stencil_front: Option<StencilOpState>,
    stencil_back: Option<StencilOpState>,

    depth_test_enabled: Option<bool>,
    depth_write_enabled: Option<bool>,
    depth_compare_op: Option<CompareOp>,
    depth_bounds_test: Option<DepthBoundTest>,

    blend: Option<ColorBlendCache>,
    viewports: Option<(Vec<ViewportEntry>, Vec<DepthRangeEntry>)>,
    index_buffer: Option<GLuint>,
    index_buffer_offset: Option<usize>,
    index_buffer_type: Option<GLenum>,

    textures: Option<Vec<GLuint>>,
    samplers: Option<Vec<GLuint>>,
    images: Option<Vec<GLuint>>,
    uniform_buffers: Option<Vec<GLuint>>,
    uniform_buffer_sizes: Option<Vec<GLsizeiptr>>,
    uniform_buffer_offsets: Option<Vec<GLintptr>>,
    shader_storage_buffers: Option<Vec<GLuint>>,
    shader_storage_buffer_sizes: Option<Vec<GLsizeiptr>>,
    shader_storage_buffer_offsets: Option<Vec<GLintptr>>,
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

pub struct IndexBuffer {
    pub buffer: GLuint,
    pub offset: usize,
    pub ty: IndexType,
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
    min: NotNan<f32>,
    max: NotNan<f32>,
}

impl StateCache {
    pub fn new(params: &ImplementationParameters) -> StateCache {
        StateCache {
            max_draw_buffers: params.max_draw_buffers,
            max_color_attachments: params.max_color_attachments,
            max_viewports: params.max_viewports,
            cull_enable: None,
            cull_mode: None,
            polygon_mode: None,
            front_face: None,
            program: None,
            vertex_array: None,
            stencil_test_enabled: None,
            stencil_front: None,
            stencil_back: None,
            depth_test_enabled: None,
            depth_write_enabled: None,
            depth_compare_op: None,
            depth_bounds_test: None,
            blend: None,
            viewports: None,
            index_buffer: None,
            index_buffer_offset: None,
            index_buffer_type: None,
            textures: None,
            samplers: None,
            images: None,
            uniform_buffers: None,
            uniform_buffer_sizes: None,
            uniform_buffer_offsets: None,
            shader_storage_buffers: None,
            shader_storage_buffer_sizes: None,
            shader_storage_buffer_offsets: None,
        }
    }

    pub fn invalidate(&mut self) {
        *self = StateCache {
            max_draw_buffers: self.max_draw_buffers,
            max_color_attachments: self.max_color_attachments,
            max_viewports: self.max_viewports,
            cull_enable: None,
            cull_mode: None,
            polygon_mode: None,
            front_face: None,
            program: None,
            vertex_array: None,
            stencil_test_enabled: None,
            stencil_front: None,
            stencil_back: None,
            depth_test_enabled: None,
            depth_write_enabled: None,
            depth_compare_op: None,
            depth_bounds_test: None,
            blend: None,
            viewports: None,
            index_buffer: None,
            index_buffer_offset: None,
            index_buffer_type: None,
            textures: None,
            samplers: None,
            images: None,
            uniform_buffers: None,
            uniform_buffer_sizes: None,
            uniform_buffer_offsets: None,
            shader_storage_buffers: None,
            shader_storage_buffer_sizes: None,
            shader_storage_buffer_offsets: None,
        };
    }

    pub fn set_program(&mut self, program: GLuint) {
        self.program.update_cached(program, || unsafe {
            gl::UseProgram(program);
        });
    }

    pub fn set_vertex_array(&mut self, vertex_array: GLuint) {
        self.vertex_array.update_cached(vertex_array, || unsafe {
            gl::BindVertexArray(vertex_array);
        });
    }

    pub fn set_all_blend(&mut self, state: &PipelineColorBlendAttachmentState) {
        let bind_all = |state: &PipelineColorBlendAttachmentState| match state {
            PipelineColorBlendAttachmentState::Disabled => unsafe { gl::Disable(gl::BLEND) },
            PipelineColorBlendAttachmentState::Enabled {
                src_color_blend_factor,
                dst_color_blend_factor,
                color_blend_op,
                src_alpha_blend_factor,
                dst_alpha_blend_factor,
                alpha_blend_op,
                color_write_mask,
            } => unsafe {
                gl::Enable(gl::BLEND);
                gl::BlendEquationSeparate(
                    blend_op_to_gl(*color_blend_op),
                    blend_op_to_gl(*alpha_blend_op),
                );
                gl::BlendFuncSeparate(
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

    pub fn set_blend_separate(&mut self, index: u32, state: &PipelineColorBlendAttachmentState) {
        let bind_separate = |index: u32, state: &PipelineColorBlendAttachmentState| match state {
            PipelineColorBlendAttachmentState::Disabled => unsafe {
                gl::Disablei(gl::BLEND, index)
            },
            PipelineColorBlendAttachmentState::Enabled {
                src_color_blend_factor,
                dst_color_blend_factor,
                color_blend_op,
                src_alpha_blend_factor,
                dst_alpha_blend_factor,
                alpha_blend_op,
                color_write_mask,
            } => unsafe {
                gl::Enablei(gl::BLEND, index);
                gl::BlendEquationSeparatei(
                    index,
                    blend_op_to_gl(*color_blend_op),
                    blend_op_to_gl(*alpha_blend_op),
                );
                gl::BlendFuncSeparatei(
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

    pub fn set_viewports(&mut self, viewports: &[Viewport]) {
        let mut should_update_viewports = false;
        let mut should_update_depth_ranges = false;

        if let Some((ref mut cur_viewports, ref mut cur_depth_ranges)) = self.viewports {
            for (i, &vp) in viewports.iter().enumerate() {
                if cur_viewports[i].x != vp.x
                    || cur_viewports[i].y != vp.y
                    || cur_viewports[i].width != vp.width
                    || cur_viewports[i].height != vp.height
                {
                    should_update_viewports = true;
                    cur_viewports[i].x = vp.x;
                    cur_viewports[i].y = vp.y;
                    cur_viewports[i].width = vp.width;
                    cur_viewports[i].height = vp.height;
                }

                if cur_depth_ranges[i].min != vp.min_depth
                    || cur_depth_ranges[i].max != vp.max_depth
                {
                    should_update_depth_ranges = true;
                    cur_depth_ranges[i].min = vp.min_depth;
                    cur_depth_ranges[i].max = vp.max_depth;
                }
            }
        } else {
            let mut new_viewports = vec![
                ViewportEntry {
                    x: 0.0.into(),
                    y: 0.0.into(),
                    width: 0.0.into(),
                    height: 0.0.into()
                };
                self.max_viewports as usize
            ];
            let mut new_depth_ranges = vec![
                DepthRangeEntry {
                    min: 0.0.into(),
                    max: 1.0.into()
                };
                self.max_viewports as usize
            ];
            for (i, &vp) in viewports.iter().enumerate() {
                new_viewports[i] = ViewportEntry {
                    x: vp.x.into(),
                    y: vp.y.into(),
                    width: vp.width.into(),
                    height: vp.height.into(),
                };
                new_depth_ranges[i] = DepthRangeEntry {
                    min: vp.min_depth.into(),
                    max: vp.max_depth.into(),
                };
            }
            self.viewports = Some((new_viewports, new_depth_ranges));
            should_update_viewports = true;
            should_update_depth_ranges = true;
        }

        // viewports cannot be None by then
        let &(ref viewports, ref depth_ranges) = &self.viewports.as_ref().unwrap();

        unsafe {
            if should_update_viewports {
                gl::ViewportArrayv(0, self.max_viewports as i32, viewports.as_ptr() as *const _);
            }

            if should_update_depth_ranges {
                gl::DepthRangeArrayv(
                    0,
                    self.max_viewports as i32,
                    depth_ranges.as_ptr() as *const _,
                );
            }
        }
    }

    pub fn set_depth_test_enable(&mut self, depth_test_enable: bool) {
        self.depth_test_enabled
            .update_cached(depth_test_enable, || unsafe {
                if depth_test_enable {
                    gl::Enable(gl::DEPTH_TEST);
                } else {
                    gl::Disable(gl::DEPTH_TEST);
                }
            })
    }

    pub fn set_depth_write_enable(&mut self, depth_write_enable: bool) {
        self.depth_write_enabled
            .update_cached(depth_write_enable, || unsafe {
                if depth_write_enable {
                    gl::DepthMask(gl::TRUE);
                } else {
                    gl::DepthMask(gl::FALSE);
                }
            })
    }

    pub fn set_depth_compare_op(&mut self, depth_compare_op: CompareOp) {
        self.depth_compare_op
            .update_cached(depth_compare_op, || unsafe {
                gl::DepthFunc(compare_op_to_gl(depth_compare_op));
            })
    }

    fn set_cull_enable(&mut self, cull_enable: bool) {
        self.cull_enable.update_cached(cull_enable, || unsafe {
            if cull_enable {
                gl::Enable(gl::CULL_FACE);
            } else {
                gl::Disable(gl::CULL_FACE);
            }
        });
    }

    pub fn set_cull_mode(&mut self, cull_mode: CullModeFlags) {
        if cull_mode == CullModeFlags::NONE {
            self.set_cull_enable(false);
        } else {
            self.set_cull_enable(true);
        }

        self.cull_mode.update_cached(cull_mode, || unsafe {
            if cull_mode.contains(CullModeFlags::FRONT_AND_BACK) {
                gl::CullFace(gl::FRONT_AND_BACK);
            } else if cull_mode.contains(CullModeFlags::FRONT) {
                gl::CullFace(gl::FRONT);
            } else if cull_mode.contains(CullModeFlags::BACK) {
                gl::CullFace(gl::BACK);
            }
        });
    }

    pub fn set_polygon_mode(&mut self, polygon_mode: PolygonMode) {
        self.polygon_mode.update_cached(polygon_mode, || unsafe {
            match polygon_mode {
                PolygonMode::Fill => gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL),
                PolygonMode::Line => gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE),
            }
        });
    }

    pub fn set_stencil_test_enabled(&mut self, enabled: bool) {
        self.stencil_test_enabled.update_cached(enabled, || unsafe {
            if enabled {
                gl::Enable(gl::STENCIL_TEST);
            } else {
                gl::Disable(gl::STENCIL_TEST);
            }
        });
    }

    /// Does not implicitly enable stencil test.
    pub fn set_stencil_op(&mut self, front: &StencilOpState, back: &StencilOpState) {
        let bind_stencil = |face: GLenum, state: &StencilOpState| unsafe {
            gl::StencilFuncSeparate(
                face,
                compare_op_to_gl(state.compare_op),
                state.reference as i32,
                state.compare_mask,
            );
            gl::StencilOpSeparate(
                face,
                stencil_op_to_gl(state.fail_op),
                stencil_op_to_gl(state.depth_fail_op),
                stencil_op_to_gl(state.pass_op),
            );
            gl::StencilMaskSeparate(face, state.write_mask);
        };

        self.stencil_front.update_cached(*front, || {
            bind_stencil(gl::FRONT, front);
        });
        self.stencil_back.update_cached(*back, || {
            bind_stencil(gl::BACK, back);
        });
    }

    pub fn set_stencil_test(&mut self, stencil_test: &StencilTest) {
        match stencil_test {
            StencilTest::Disabled => self.set_stencil_test_enabled(false),
            StencilTest::Enabled { front, back } => {
                self.set_stencil_test_enabled(true);
                self.set_stencil_op(front, back);
            }
        }
    }

    pub fn set_uniform_buffers(
        &mut self,
        buffers: &[GLuint],
        buffer_offsets: &[GLintptr],
        buffer_sizes: &[GLintptr],
    ) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe {
            let count = buffers.len();
            if count != 0 {
                gl::BindBuffersRange(
                    gl::UNIFORM_BUFFER,
                    0,
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
        buffers: &[GLuint],
        buffer_offsets: &[GLintptr],
        buffer_sizes: &[GLintptr],
    ) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe {
            let count = buffers.len();
            if count != 0 {
                gl::BindBuffersRange(
                    gl::SHADER_STORAGE_BUFFER,
                    0,
                    count as i32,
                    buffers.as_ptr(),
                    buffer_offsets.as_ptr(),
                    buffer_sizes.as_ptr(),
                );
            }
        }
    }

    pub fn set_samplers(&mut self, samplers: &[GLuint]) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe { gl::BindSamplers(0, samplers.len() as i32, samplers.as_ptr()) }
    }

    pub fn set_textures(&mut self, textures: &[GLuint]) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe { gl::BindTextures(0, textures.len() as i32, textures.as_ptr()) }
    }

    pub fn set_vertex_buffers(
        &mut self,
        buffers: &[GLuint],
        buffer_offsets: &[GLintptr],
        buffer_strides: &[GLsizei],
    ) {
        // passthrough, for now
        // may do a comparison, or a quick diff in the future
        unsafe {
            let count = buffers.len();
            if count != 0 {
                gl::BindVertexBuffers(
                    0,
                    count as i32,
                    buffers.as_ptr(),
                    buffer_offsets.as_ptr(),
                    buffer_strides.as_ptr(),
                )
            }
        }
    }

    pub fn set_index_buffer(&mut self, buffer: GLuint, offset: usize, ty: IndexType) {
        self.index_buffer.update_cached(buffer, || unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, buffer);
        });

        self.index_buffer_offset = Some(offset);
        self.index_buffer_type = Some(match ty {
            IndexType::U16 => gl::UNSIGNED_SHORT,
            IndexType::U32 => gl::UNSIGNED_INT,
        });
    }

    //pub fn set_blend_mode(&mut self)
}
