use ordered_float::NotNan;
use std::error::Error;
use std::fmt;
use unreachable::UncheckedOptionExt;

use crate::renderer::backend::gl::{
    api as gl,
    api::types::*,
    format::GlFormatInfo,
    shader::{create_specialized_spirv_shader, ShaderCreationError, ShaderModule},
    state::StateCache,
    Arena, OpenGlBackend,
};
use crate::renderer::{
    GraphicsPipelineCreateInfo, GraphicsPipelineShaderStages, LogicOp,
    PipelineColorBlendAttachmentState, PipelineColorBlendAttachments,
    PipelineColorBlendStateCreateInfo, PipelineDepthStencilStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineLayoutCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo, PipelineScissors,
    SamplerDescription, ShaderStageFlags, VertexInputAttributeDescription,
    VertexInputBindingDescription,
};

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum BindingSpace {
    UniformBuffer,
    ShaderStorageBuffer,
    AtomicCounterBuffer,
    Texture,
    Image,
    Empty,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct BindingLocation {
    pub space: BindingSpace,
    pub location: u32,
}

#[derive(Clone, Debug)]
pub struct DescriptorMap {
    pub sets: Vec<Vec<BindingLocation>>,
}

impl DescriptorMap {
    pub fn get_binding_location(&self, set: u32, binding: u32) -> Option<BindingLocation> {
        self.sets.get(set as usize).and_then(|set| {
            set.get(binding as usize).and_then(|loc| {
                if loc.space == BindingSpace::Empty {
                    None
                } else {
                    Some(*loc)
                }
            })
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct StaticSamplerEntry {
    pub texture_binding_range: (u32, u32),
    pub description: SamplerDescription,
}

#[derive(Clone, Debug)]
pub struct GraphicsPipelineCreateInfoAdditional {
    // these two members should really be slices instead of owned data
    // but that's impossible until rust supports generic associated types
    // https://github.com/rust-lang/rust/issues/44265
    pub descriptor_map: DescriptorMap,
    pub static_samplers: Vec<StaticSamplerEntry>,
}

//--------------------------------------------------------------------------------------------------
fn create_vertex_array_object(attribs: &[VertexInputAttributeDescription]) -> GLuint {
    let mut vao = 0;
    unsafe {
        gl::CreateVertexArrays(1, &mut vao);
    }

    for a in attribs.iter() {
        unsafe {
            gl::EnableVertexArrayAttrib(vao, a.location);
            let fmtinfo = a.format.get_format_info();
            let normalized = fmtinfo.is_normalized() as u8;
            let size = fmtinfo.num_components() as i32;
            let glfmt = GlFormatInfo::from_format(a.format);
            let ty = glfmt.upload_ty;

            gl::VertexArrayAttribFormat(vao, a.location, size, ty, normalized, a.offset);
            gl::VertexArrayAttribBinding(vao, a.location, a.binding);
        }
    }

    vao
}

fn link_program(obj: GLuint) -> Result<GLuint, String> {
    unsafe {
        gl::LinkProgram(obj);
        let mut status = 0;
        let mut log_size = 0;
        gl::GetProgramiv(obj, gl::LINK_STATUS, &mut status);
        gl::GetProgramiv(obj, gl::INFO_LOG_LENGTH, &mut log_size);
        //trace!("LINK_STATUS: log_size: {}, status: {}", log_size, status);
        if status != gl::TRUE as GLint {
            let mut log_buf = Vec::with_capacity(log_size as usize);
            gl::GetProgramInfoLog(
                obj,
                log_size,
                &mut log_size,
                log_buf.as_mut_ptr() as *mut i8,
            );
            log_buf.set_len(log_size as usize);
            Err(String::from_utf8(log_buf).unwrap())
        } else {
            Ok(obj)
        }
    }
}

//--------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct ProgramCreationError(String);

impl fmt::Display for ProgramCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for ProgramCreationError {}

impl From<ShaderCreationError> for ProgramCreationError {
    fn from(err: ShaderCreationError) -> Self {
        ProgramCreationError(err.0)
    }
}

fn create_graphics_program(
    vertex_shader: &ShaderModule,
    fragment_shader: Option<&ShaderModule>,
    geometry_shader: Option<&ShaderModule>,
    tess_control_shader: Option<&ShaderModule>,
    tess_eval_shader: Option<&ShaderModule>,
) -> Result<GLuint, ProgramCreationError> {
    let spirv = vertex_shader.spirv.is_some();

    if fragment_shader.map_or(false, |s| s.spirv.is_some() != spirv)
        || geometry_shader.map_or(false, |s| s.spirv.is_some() != spirv)
        || tess_control_shader.map_or(false, |s| s.spirv.is_some() != spirv)
        || tess_eval_shader.map_or(false, |s| s.spirv.is_some() != spirv)
    {
        return Err(ProgramCreationError(
            "cannot mix SPIR-V and GLSL shaders".into(),
        ));
    }

    let (vs, fs, gs, tcs, tes) = if spirv {
        // SPIR-V path
        let vs = create_specialized_spirv_shader(ShaderStageFlags::VERTEX, "main", unsafe {
            vertex_shader.spirv.as_ref().unchecked_unwrap()
        })?;
        let fs = if let Some(s) = fragment_shader {
            create_specialized_spirv_shader(ShaderStageFlags::FRAGMENT, "main", unsafe {
                s.spirv.as_ref().unchecked_unwrap()
            })?
            .into()
        } else {
            None
        };
        let gs = if let Some(s) = geometry_shader {
            create_specialized_spirv_shader(ShaderStageFlags::GEOMETRY, "main", unsafe {
                s.spirv.as_ref().unchecked_unwrap()
            })?
            .into()
        } else {
            None
        };
        let tcs = if let Some(s) = tess_control_shader {
            create_specialized_spirv_shader(ShaderStageFlags::TESS_CONTROL, "main", unsafe {
                s.spirv.as_ref().unchecked_unwrap()
            })?
            .into()
        } else {
            None
        };
        let tes = if let Some(s) = tess_eval_shader {
            create_specialized_spirv_shader(ShaderStageFlags::TESS_EVAL, "main", unsafe {
                s.spirv.as_ref().unchecked_unwrap()
            })?
            .into()
        } else {
            None
        };
        (vs, fs, gs, tcs, tes)
    } else {
        // GLSL path
        (
            vertex_shader.obj,
            fragment_shader.map(|s| s.obj),
            geometry_shader.map(|s| s.obj),
            tess_control_shader.map(|s| s.obj),
            tess_eval_shader.map(|s| s.obj),
        )
    };

    unsafe {
        let program = gl::CreateProgram();

        gl::AttachShader(program, vs);
        if let Some(s) = fs {
            gl::AttachShader(program, s);
        }
        if let Some(s) = gs {
            gl::AttachShader(program, s);
        }
        if let Some(s) = tcs {
            gl::AttachShader(program, s);
        }
        if let Some(s) = tes {
            gl::AttachShader(program, s);
        }

        link_program(program).map_err(|log| {
            // cleanup
            gl::DeleteProgram(program);
            if spirv {
                gl::DeleteShader(vs);
                if let Some(s) = fs {
                    gl::DeleteShader(s);
                }
                if let Some(s) = gs {
                    gl::DeleteShader(s);
                }
                if let Some(s) = tcs {
                    gl::DeleteShader(s);
                }
                if let Some(s) = tes {
                    gl::DeleteShader(s);
                }
            }

            ProgramCreationError(format!("program link error: {}", log))
        })?;

        if spirv {
            // cleanup
            gl::DeleteShader(vs);
            if let Some(s) = fs {
                gl::DeleteShader(s);
            }
            if let Some(s) = gs {
                gl::DeleteShader(s);
            }
            if let Some(s) = tcs {
                gl::DeleteShader(s);
            }
            if let Some(s) = tes {
                gl::DeleteShader(s);
            }
        }

        Ok(program)
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum PipelineColorBlendAttachmentsOwned {
    All(PipelineColorBlendAttachmentState),
    Separate(Vec<PipelineColorBlendAttachmentState>),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PipelineColorBlendStateOwned {
    pub logic_op: Option<LogicOp>,
    pub attachments: PipelineColorBlendAttachmentsOwned,
    pub blend_constants: [NotNan<f32>; 4],
}

#[derive(Clone, Debug)]
pub struct GraphicsPipeline {
    pub(super) rasterization_state: PipelineRasterizationStateCreateInfo,
    pub(super) depth_stencil_state: PipelineDepthStencilStateCreateInfo,
    pub(super) multisample_state: PipelineMultisampleStateCreateInfo,
    pub(super) input_assembly_state: PipelineInputAssemblyStateCreateInfo,
    pub(super) vertex_input_bindings: Vec<VertexInputBindingDescription>,
    pub(super) color_blend_state: PipelineColorBlendStateOwned,
    pub(super) descriptor_map: DescriptorMap,
    pub(super) static_samplers: Vec<StaticSamplerEntry>,
    pub(super) program: GLuint,
    pub(super) vao: GLuint,
}

impl GraphicsPipeline {
    pub fn descriptor_map(&self) -> &DescriptorMap {
        &self.descriptor_map
    }

    pub fn static_samplers(&self) -> &[StaticSamplerEntry] {
        &self.static_samplers
    }

    pub fn vertex_input_bindings(&self) -> &[VertexInputBindingDescription] {
        &self.vertex_input_bindings
    }
}

pub fn create_graphics_pipeline_internal<'a>(
    arena: &'a Arena,
    create_info: &GraphicsPipelineCreateInfo<OpenGlBackend>,
) -> &'a GraphicsPipeline {
    let program = {
        let vs = create_info.shader_stages.vertex.0;
        let fs = create_info.shader_stages.fragment.map(|s| s.0);
        let gs = create_info.shader_stages.geometry.map(|s| s.0);
        let tcs = create_info.shader_stages.tess_control.map(|s| s.0);
        let tes = create_info.shader_stages.tess_eval.map(|s| s.0);
        create_graphics_program(vs, fs, gs, tcs, tes).expect("failed to create program")
    };

    //assert_eq!(vertex_shader.stage, ShaderStageFlags::VERTEX);
    let vao = create_vertex_array_object(create_info.vertex_input_state.attributes);

    let color_blend_state = PipelineColorBlendStateOwned {
        logic_op: create_info.color_blend_state.logic_op,
        attachments: match create_info.color_blend_state.attachments {
            PipelineColorBlendAttachments::All(a) => PipelineColorBlendAttachmentsOwned::All(*a),
            PipelineColorBlendAttachments::Separate(a) => {
                PipelineColorBlendAttachmentsOwned::Separate(a.to_vec())
            }
        },
        blend_constants: create_info.color_blend_state.blend_constants,
    };

    let g = GraphicsPipeline {
        rasterization_state: *create_info.rasterization_state,
        depth_stencil_state: *create_info.depth_stencil_state,
        multisample_state: *create_info.multisample_state,
        input_assembly_state: *create_info.input_assembly_state,
        vertex_input_bindings: create_info.vertex_input_state.bindings.to_vec(),
        program,
        vao,
        descriptor_map: create_info.additional.descriptor_map.clone(),
        static_samplers: create_info.additional.static_samplers.clone(),
        color_blend_state,
    };

    arena.graphics_pipelines.alloc(g)
}

impl GraphicsPipeline {
    pub fn bind(&self, state_cache: &mut StateCache) {
        state_cache.set_program(self.program);
        state_cache.set_vertex_array(self.vao);
        state_cache.set_cull_mode(self.rasterization_state.cull_mode);
        state_cache.set_polygon_mode(self.rasterization_state.polygon_mode);
        state_cache.set_stencil_test(&self.depth_stencil_state.stencil_test);
        match self.color_blend_state.attachments {
            PipelineColorBlendAttachmentsOwned::All(ref state) => state_cache.set_all_blend(state),
            PipelineColorBlendAttachmentsOwned::Separate(ref states) => {
                for (i, s) in states.iter().enumerate() {
                    state_cache.set_blend_separate(i as u32, s);
                }
            }
        }
    }
}
