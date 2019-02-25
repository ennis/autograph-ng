use crate::{
    api::{types::*, Gl},
    backend::{GlArena, OpenGlBackend},
    command::StateCache,
};
use autograph_render::{
    image::SamplerDescription,
    pipeline::{
        ColorBlendAttachmentState, ColorBlendAttachments, DepthStencilState, InputAssemblyState,
        LogicOp, MultisampleState, RasterisationState, VertexInputBindingDescription,
    },
};
use ordered_float::NotNan;

mod arguments;
mod program;
mod shader;
mod vao;

use self::{program::create_graphics_program, vao::create_vertex_array_object};

pub(crate) use self::{
    arguments::{GlArgumentBlock, GlSignature, StateBlock},
    shader::{DescriptorMap, GlShaderModule},
};
use autograph_render::{
    pipeline::{
        GraphicsPipelineCreateInfo, ScissorsOwned, SignatureDescription,
        VertexInputAttributeDescription, VertexInputRate, ViewportsOwned,
    },
    vertex::VertexLayout,
};

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
    pub(crate) viewports: ViewportsOwned,
    pub(crate) scissors: ScissorsOwned,
    pub(crate) program: GLuint,
    pub(crate) vao: GLuint,
}

impl GlGraphicsPipeline {
    pub(crate) fn descriptor_map(&self) -> &DescriptorMap {
        &self.descriptor_map
    }

    /*pub(crate) fn vertex_input_bindings(&self) -> &[VertexInputBindingDescription] {
        &self.vertex_input_bindings
    }*/
}

/// Converts a sequence of VertexLayouts (one for each vertex buffer) into binding descriptions
/// and vertex attribute descriptions.
///
/// This function generates vertex attributes for each element in all layouts,
/// and laid out sequentially : i.e. if buffer #0 has 4 elements,
/// and buffer #1 has 2 elements, then 6 attributes will be generated:
/// attributes 0..=3 will map to vertex buffer 0 and attributes 4..=5 will map to vertex buffer 1.
pub(crate) fn build_vertex_input_interface(
    buffer_layouts: &[VertexLayout],
) -> (
    Vec<VertexInputBindingDescription>,
    Vec<VertexInputAttributeDescription>,
) {
    let mut input_bindings = Vec::new();
    let mut input_attribs = Vec::new();

    let mut location = 0;

    for (binding, &layout) in buffer_layouts.iter().enumerate() {
        input_bindings.push(VertexInputBindingDescription {
            binding: binding as u32,
            stride: layout.stride as u32,
            input_rate: VertexInputRate::Vertex,
        });

        for &attrib in layout.elements.iter() {
            input_attribs.push(VertexInputAttributeDescription {
                location,
                binding: binding as u32,
                format: attrib.format,
                offset: attrib.offset,
            });
            location += 1;
        }
    }

    (input_bindings, input_attribs)
}

fn collect_vertex_layouts<'a>(sig: &'a SignatureDescription<'a>, out: &mut Vec<VertexLayout<'a>>) {
    for &i in sig.inherited {
        collect_vertex_layouts(i, out);
    }
    out.extend(sig.vertex_layouts.iter().cloned());
}

//--------------------------------------------------------------------------------------------------
pub(crate) unsafe fn create_graphics_pipeline_internal<'a>(
    gl: &Gl,
    arena: &'a GlArena,
    _root_signature: &'a GlSignature,
    root_signature_description: &SignatureDescription,
    ci: &GraphicsPipelineCreateInfo<'a, '_, OpenGlBackend>,
) -> &'a GlGraphicsPipeline {
    let (program, descriptor_map) = {
        let vs = ci.shader_stages.vertex.inner();
        let fs = ci.shader_stages.fragment.map(|s| s.inner());
        let gs = ci.shader_stages.geometry.map(|s| s.inner());
        let tcs = ci.shader_stages.tess_control.map(|s| s.inner());
        let tes = ci.shader_stages.tess_eval.map(|s| s.inner());
        create_graphics_program(gl, vs, fs, gs, tcs, tes).expect("failed to create program")
    };

    // collect vertex layouts
    let mut vertex_layouts = Vec::new();
    collect_vertex_layouts(root_signature_description, &mut vertex_layouts);

    let (vertex_input_bindings, vertex_input_attribs) =
        build_vertex_input_interface(&vertex_layouts);
    let vao = create_vertex_array_object(gl, &vertex_input_attribs);

    /*    // count number of viewports
    let num_viewports = match ci.viewport_state.viewports {
        Viewports::Dynamic => root_signature_description.count_viewports(),
        Viewports::Static(viewports) => {
            //unimplemented!(); // TODO static viewports
            viewports.len()
        }
    };*/

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
        rasterization_state: ci.rasterization_state,
        depth_stencil_state: ci.depth_stencil_state,
        multisample_state: ci.multisample_state,
        input_assembly_state: ci.input_assembly_state,
        vertex_input_bindings,
        program,
        vao,
        descriptor_map,
        color_blend_state,
        viewports: ci.viewport_state.viewports.into(),
        scissors: ci.viewport_state.scissors.into(),
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
        // static viewports & scissors
        if let ViewportsOwned::Static(ref vp) = &self.viewports {
            state_cache.set_viewports(gl, vp);
        }
        if let ScissorsOwned::Static(ref sc) = &self.scissors {
            state_cache.set_scissors(gl, sc);
        }
    }
}
