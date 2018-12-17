use ordered_float::NotNan;
use std::error::Error;
use std::fmt;
use unreachable::UncheckedOptionExt;

use crate::renderer::backend::gl::{
    api as gl,
    api::types::*,
    format::GlFormatInfo,
    shader::{create_specialized_spirv_shader, DescriptorMapBuilder, ShaderCreationError, ShaderModule, translate_spirv_to_gl_flavor},
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
pub struct FlatBinding {
    pub space: BindingSpace,
    pub location: u32,
}

impl FlatBinding {
    pub fn new(space: BindingSpace, location: u32) -> FlatBinding {
        FlatBinding { space, location }
    }
}

#[derive(Clone, Debug)]
pub struct DescriptorMap(pub Vec<Vec<FlatBinding>>);

impl DescriptorMap {
    pub fn get_binding_location(&self, set: u32, binding: u32) -> Option<FlatBinding> {
        self.0.get(set as usize).and_then(|set| {
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
    pub tex_range: (u32, u32),
    pub desc: SamplerDescription,
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
    vert: &ShaderModule,
    frag: Option<&ShaderModule>,
    geom: Option<&ShaderModule>,
    tessctl: Option<&ShaderModule>,
    tesseval: Option<&ShaderModule>,
    user_dm: DescriptorMap,
) -> Result<(GLuint, DescriptorMap), ProgramCreationError>
{
    let spirv = vert.spirv.is_some();
    let mut dm = user_dm;

    // Verify that we are not mixing GLSL and SPIR-V shaders
    if frag.map_or(false, |s| s.spirv.is_some() != spirv)
        || geom.map_or(false, |s| s.spirv.is_some() != spirv)
        || tessctl.map_or(false, |s| s.spirv.is_some() != spirv)
        || tesseval.map_or(false, |s| s.spirv.is_some() != spirv)
    {
        return Err(ProgramCreationError(
            "cannot mix both SPIR-V and GLSL shaders".into(),
        ));
    }

    let (vs, fs, gs, tcs, tes) = if spirv {
        // SPIR-V path: translate to GL dialect and at the same time build
        // the descriptor map
        let mut dmb = DescriptorMapBuilder::new();
        let vert = vert.spirv.as_ref().unwrap();
        let frag = frag.map(|s| s.spirv.as_ref().unwrap());
        let geom = geom.map(|s| s.spirv.as_ref().unwrap());
        let tessctl = tessctl.map(|s| s.spirv.as_ref().unwrap());
        let tesseval = tesseval.map(|s| s.spirv.as_ref().unwrap());

        let vs = {
            let vert = translate_spirv_to_gl_flavor(vert, ShaderStageFlags::VERTEX, &mut dmb);
            create_specialized_spirv_shader(ShaderStageFlags::VERTEX, "main", &vert)?
        };

        let fs = if let Some(s) = frag {
            let s = translate_spirv_to_gl_flavor(s, ShaderStageFlags::FRAGMENT, &mut dmb);
            create_specialized_spirv_shader(ShaderStageFlags::FRAGMENT, "main", &s)?.into()
        } else {
            None
        };

        let gs = if let Some(s) = geom {
            let s = translate_spirv_to_gl_flavor(s, ShaderStageFlags::GEOMETRY, &mut dmb);
            create_specialized_spirv_shader(ShaderStageFlags::GEOMETRY, "main", &s)?.into()
        } else {
            None
        };
        let tcs = if let Some(s) = tessctl {
            let s = translate_spirv_to_gl_flavor(s, ShaderStageFlags::TESS_CONTROL, &mut dmb);
            create_specialized_spirv_shader(ShaderStageFlags::TESS_CONTROL, "main", &s)?.into()
        } else {
            None
        };
        let tes = if let Some(s) = tesseval {
            let s = translate_spirv_to_gl_flavor(s, ShaderStageFlags::TESS_EVAL, &mut dmb);
            create_specialized_spirv_shader(ShaderStageFlags::TESS_EVAL, "main", &s)?.into()
        } else {
            None
        };

        // overwrite user-provided descriptor map
        dm = dmb.into();
        (vs, fs, gs, tcs, tes)

    } else {
        // GLSL path
        (
            vert.obj,
            frag.map(|s| s.obj),
            geom.map(|s| s.obj),
            tessctl.map(|s| s.obj),
            tesseval.map(|s| s.obj),
        )
    };

    // create program, attach shaders, and link program
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
            // the SPIR-V path has generated new shader objects: don't leak them
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

        Ok((program, dm))
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

//--------------------------------------------------------------------------------------------------
pub fn create_graphics_pipeline_internal<'a>(
    arena: &'a Arena,
    ci: &GraphicsPipelineCreateInfo<OpenGlBackend>,
) -> &'a GraphicsPipeline
{
    let (program, descriptor_map) = {
        let vs = ci.shader_stages.vertex.0;
        let fs = ci.shader_stages.fragment.map(|s| s.0);
        let gs = ci.shader_stages.geometry.map(|s| s.0);
        let tcs = ci.shader_stages.tess_control.map(|s| s.0);
        let tes = ci.shader_stages.tess_eval.map(|s| s.0);
        create_graphics_program(vs, fs, gs, tcs, tes, ci.additional.descriptor_map.clone()).expect("failed to create program")
    };

    //assert_eq!(vertex_shader.stage, ShaderStageFlags::VERTEX);
    let vao = create_vertex_array_object(ci.vertex_input_state.attributes);

    let color_blend_state = PipelineColorBlendStateOwned {
        logic_op: ci.color_blend_state.logic_op,
        attachments: match ci.color_blend_state.attachments {
            PipelineColorBlendAttachments::All(a) => PipelineColorBlendAttachmentsOwned::All(*a),
            PipelineColorBlendAttachments::Separate(a) => {
                PipelineColorBlendAttachmentsOwned::Separate(a.to_vec())
            }
        },
        blend_constants: ci.color_blend_state.blend_constants,
    };

    let g = GraphicsPipeline {
        rasterization_state: *ci.rasterization_state,
        depth_stencil_state: *ci.depth_stencil_state,
        multisample_state: *ci.multisample_state,
        input_assembly_state: *ci.input_assembly_state,
        vertex_input_bindings: ci.vertex_input_state.bindings.to_vec(),
        program,
        vao,
        descriptor_map,
        static_samplers: ci.additional.static_samplers.clone(),
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
