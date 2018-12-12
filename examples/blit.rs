extern crate gfx2;
#[macro_use]
extern crate log;
extern crate nalgebra_glm as glm;

use std::env;
use std::mem;

use gfx2::app::*;
use gfx2::renderer;
use gfx2::renderer::backend::gl as gl_backend;
use gfx2::renderer::*;


//--------------------------------------------------------------------------------------------------
type Backend = gl_backend::OpenGlBackend;
type Buffer<'a, T: BufferData + ?Sized> = renderer::Buffer<'a, Backend, T>;
type BufferTypeless<'a> = renderer::BufferTypeless<'a, Backend>;
type Image<'a> = renderer::Image<'a, Backend>;
type Framebuffer<'a> = renderer::Framebuffer<'a, Backend>;
type DescriptorSet<'a> = renderer::DescriptorSet<'a, Backend>;
type DescriptorSetLayout<'a> = renderer::DescriptorSetLayout<'a, Backend>;
type GraphicsPipeline<'a> = renderer::GraphicsPipeline<'a, Backend>;

//--------------------------------------------------------------------------------------------------
#[derive(Copy,Clone)]
#[repr(C)]
pub struct Vertex {
    pub pos: glm::Vec2,
    pub tex: glm::Vec2,
}

#[derive(Copy,Clone)]
#[repr(C)]
pub struct Uniforms {
    pub transform: glm::Mat3x4
}

pub struct PerObject<'a> {
    pub uniforms: Buffer<'a, Uniforms>,
    pub image: Image<'a>,
}

impl<'a> DescriptorSetInterface<'a, Backend> for PerObject<'a> {
    const INTERFACE: DescriptorSetDescription<'static> = DescriptorSetDescription { descriptors: &[
        DescriptorSetLayoutBinding {
            binding: 0,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            descriptor_type: DescriptorType::UniformBuffer,
            count: 1,
            tydesc: None,
        },
        DescriptorSetLayoutBinding {
            binding: 1,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            descriptor_type: DescriptorType::SampledImage,
            count: 1,
            tydesc: None,
        },]
    };

    fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>) {
        visitor.visit_buffer(0, self.uniforms.into(), 0, mem::size_of::<Uniforms>());
        visitor.visit_sampled_image(1, self.image, SamplerDescription::LINEAR_MIPMAP_LINEAR);
    }
}

pub struct Blit<'a> {
    pub framebuffer: Framebuffer<'a>,
    pub per_object: DescriptorSet<'a>,
    pub viewport: Viewport,
    pub vertex_buffer: Buffer<'a, [Vertex]>,
}

impl<'a> PipelineInterface<'a, Backend> for Blit<'a> {
    const VERTEX_INPUT_INTERFACE: &'static [VertexInputBufferDescription<'static>] = &[];
    const FRAGMENT_OUTPUT_INTERFACE: &'static [FragmentOutputDescription] = &[];
    const DESCRIPTOR_SET_INTERFACE: &'static [DescriptorSetDescription<'static>] = &[];

    fn do_visit(&self, visitor: &mut PipelineInterfaceVisitor<'a, Backend>) {
        visitor.visit_dynamic_viewports(&[self.viewport]);
        visitor.visit_vertex_buffers(&[self.vertex_buffer.into()]);
        visitor.visit_framebuffer(self.framebuffer);
        visitor.visit_descriptor_sets(&[self.per_object]);
    }
}

//--------------------------------------------------------------------------------------------------
struct PipelineAndLayout<'a> {
    blit_pipeline: GraphicsPipeline<'a>,
    descriptor_set_layout: DescriptorSetLayout<'a>,
}

fn create_pipelines<'a>(arena: &'a Arena<Backend>) -> PipelineAndLayout<'a> {
    // load pipeline file
    let pp = gl_backend::PipelineDescriptionFile::load(arena, "tests/data/shaders/blit.glsl")
        .unwrap();

    let shader_stages = GraphicsPipelineShaderStages {
        vertex: pp.modules.vs.unwrap(),
        geometry: pp.modules.gs,
        fragment: pp.modules.fs,
        tess_eval: pp.modules.tes,
        tess_control: pp.modules.tcs,
    };

    let vertex_input_state = PipelineVertexInputStateCreateInfo {
        bindings: &[VertexInputBindingDescription {
            binding: 0,
            stride: 44,
            input_rate: VertexInputRate::Vertex,
        }],
        attributes: pp
            .preprocessed
            .vertex_attributes
            .as_ref()
            .unwrap()
            .as_slice(),
    };

    let viewport_state = PipelineViewportStateCreateInfo {
        viewports: PipelineViewports::Dynamic,
        scissors: PipelineScissors::Dynamic,
    };

    let rasterization_state = PipelineRasterizationStateCreateInfo::DEFAULT;
    let depth_stencil_state = PipelineDepthStencilStateCreateInfo::default();
    let color_blend_state = PipelineColorBlendStateCreateInfo {
        attachments: PipelineColorBlendAttachments::All(
            &PipelineColorBlendAttachmentState::DISABLED,
        ),
        blend_constants: [0.0.into(); 4],
        logic_op: None,
    };

    let multisample_state = PipelineMultisampleStateCreateInfo::default();

    let input_assembly_state = PipelineInputAssemblyStateCreateInfo {
        topology: PrimitiveTopology::TriangleList,
        primitive_restart_enable: false,
    };

    let descriptor_set_layout = arena.create_descriptor_set_layout(&[
        // camera parameters
        DescriptorSetLayoutBinding {
            binding: 0,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            descriptor_type: DescriptorType::UniformBuffer,
            count: 1,
            tydesc: None,
        },
    ]);

    let descriptor_set_layouts = [
        per_frame_descriptor_set_layout,
        per_object_descriptor_set_layout,
    ];

    let pipeline_layout = PipelineLayoutCreateInfo {
        descriptor_set_layouts: descriptor_set_layouts.as_ref(),
    };

    let attachment_layout = AttachmentLayoutCreateInfo {
        input_attachments: &[],
        depth_attachment: None,
        color_attachments: &[
            AttachmentDescription {
                format: Format::R8G8B8A8_SRGB,
                samples: 1,
            },
        ],
    };

    let additional = gl_backend::GraphicsPipelineCreateInfoAdditional {
        descriptor_map: pp.descriptor_map.clone(),
        static_samplers: pp.preprocessed.static_samplers.clone(),
    };

    let gci = GraphicsPipelineCreateInfo {
        shader_stages: &shader_stages,
        vertex_input_state: &vertex_input_state,
        viewport_state: &viewport_state,
        rasterization_state: &rasterization_state,
        multisample_state: &multisample_state,
        depth_stencil_state: &depth_stencil_state,
        input_assembly_state: &input_assembly_state,
        color_blend_state: &color_blend_state,
        dynamic_state: DynamicStateFlags::VIEWPORT,
        pipeline_layout: &pipeline_layout,
        attachment_layout: &attachment_layout,
        additional: &additional,
    };

    PipelineAndLayout {
        blit_pipeline: arena.create_graphics_pipeline(&gci),
        descriptor_set_layout,
    }
}

//--------------------------------------------------------------------------------------------------
