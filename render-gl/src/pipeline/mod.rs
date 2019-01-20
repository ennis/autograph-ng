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
use autograph_render::traits;
use ordered_float::NotNan;

mod program;
mod shader;
mod vao;

use self::program::create_graphics_program;
use self::vao::create_vertex_array_object;

pub(crate) use self::shader::BindingSpace;
pub(crate) use self::shader::DescriptorMap;
pub(crate) use self::shader::GlShaderModule;
use crate::DowncastPanic;

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

impl traits::GraphicsPipeline for GlGraphicsPipeline {}

//--------------------------------------------------------------------------------------------------
pub(crate) fn create_graphics_pipeline_internal<'a>(
    gl: &Gl,
    arena: &'a GlArena,
    ci: &GraphicsPipelineCreateInfoTypeless,
) -> &'a GlGraphicsPipeline {
    let (program, descriptor_map) = {
        let vs = ci
            .shader_stages
            .vertex
            .0
            .downcast_ref_unwrap::<GlShaderModule>();
        let fs = ci
            .shader_stages
            .fragment
            .map(|s| s.0.downcast_ref_unwrap::<GlShaderModule>());
        let gs = ci
            .shader_stages
            .geometry
            .map(|s| s.0.downcast_ref_unwrap::<GlShaderModule>());
        let tcs = ci
            .shader_stages
            .tess_control
            .map(|s| s.0.downcast_ref_unwrap::<GlShaderModule>());
        let tes = ci
            .shader_stages
            .tess_eval
            .map(|s| s.0.downcast_ref_unwrap::<GlShaderModule>());
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
