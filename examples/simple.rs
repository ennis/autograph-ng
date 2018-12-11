extern crate gfx2;
#[macro_use]
extern crate log;
extern crate nalgebra_glm as glm;

use std::env;

use gfx2::app::*;
use gfx2::renderer::backend::gl as gl_backend;
use gfx2::renderer::*;

type Backend = gl_backend::OpenGlBackend;
type Image = <Backend as RendererBackend>::Image;
type Framebuffer = <Backend as RendererBackend>::Framebuffer;
type Buffer = <Backend as RendererBackend>::Buffer;
type DescriptorSet = <Backend as RendererBackend>::DescriptorSet;
type DescriptorSetLayout = <Backend as RendererBackend>::DescriptorSetLayout;
type GraphicsPipeline = <Backend as RendererBackend>::GraphicsPipeline;

//--------------------------------------------------------------------------------------------------

/*
define_sort_key! {

    sequence:3 {
        MAIN => user_defined:25, pass_immediate:4,
        UI => user_defined,

        PRESENT => user_defined:25, pass_immediate:4
    }

    [sequence:3, layer:8, depth:16, pass_immediate:4],
    [opaque:3 = 3, layer:8, depth:16, pass_immediate:4],
    [shadow:3 = 1, view: 6, layer:8, depth:16, pass_immediate:4]

    sequence,objgroup,comp-pass(pre,draw,post),effect,effect-pass(pre,draw,post)
}

sequence_id!{ opaque, layer=group_id, depth=d, pass_immediate=0 }*/

pub struct RenderKey(u64);

impl RenderKey {}

#[repr(C)]
struct CameraParameters {
    view_matrix: glm::Mat4,
    proj_matrix: glm::Mat4,
    view_proj_matrix: glm::Mat4,
    inv_proj_matrix: glm::Mat4,
    view_proj_matrix_velocity: glm::Mat4,
    prev_view_proj_matrix_velocity: glm::Mat4,
    taa_offset: glm::Vec2,
}

#[repr(C)]
struct ObjectParameters {
    model_matrix: glm::Mat4,
    prev_model_matrix: glm::Mat4,
    object_id: i32,
}

/*
#[derive(FragmentOutputInterface)]
struct SamplePipelineAttachments
{
    #[color_attachment(0)]
    diffuse: ImageHandle,
    #[color_attachment(1)]
    normals: ImageHandle,
    #[color_attachment(2)]
    object_id: ImageHandle,
    #[color_attachment(3)]
    velocity: ImageHandle,
    #[depth_attachment]
    depth: ImageHandle
}

#[derive(VertexLayout)]
struct SamplePipelineVertex
{
    position: glm::Vec3,
    normal: glm::Vec3,
    tangent: glm::Vec3,
    texcoords: glm::Vec2,
}

#[derive(PipelineInterface)]
struct SamplePipelineInterface
{
    #[descriptor_set(0)]
    per_frame: DescriptorSet<PerFrameDescriptors>,
    #[descriptor_set(1)]
    per_object: DescriptorSet<PerObjectDescriptors>,
    #[viewport]
    viewport: Viewport,
    #[attachments]
    attachments: SamplePipelineAttachments,
    #[vertex_input(0)]
    vertex_buffer: VertexBuffer<SamplePipelineVertex>,
}
*/

//--------------------------------------------------------------------------------------------------
struct PerFrameUniforms<'a, R: RendererBackend> {
    camera_params: &'a R::Buffer,
    test: &'a R::Buffer,
}

// SHOULD BE AUTOMATICALLY DERIVED
impl<'a, R: RendererBackend> DescriptorSetInterface<'a, R> for PerFrameUniforms<'a, R> {
    const INTERFACE: DescriptorSetDescription<'static> = DescriptorSetDescription {
        descriptors: &[DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
            tydesc: None,
        }],
    };

    fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>) {
        visitor.visit_buffer(0, self.camera_params);
    }
}

//--------------------------------------------------------------------------------------------------
struct PerObjectUniforms<'a, R: RendererBackend> {
    obj_params: &'a R::Buffer,
}

// SHOULD BE AUTOMATICALLY DERIVED
impl<'a, R: RendererBackend> DescriptorSetInterface<'a, R> for PerObjectUniforms<'a, R> {
    const INTERFACE: DescriptorSetDescription<'static> = DescriptorSetDescription {
        descriptors: &[DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
            tydesc: None,
        }],
    };

    fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>) {
        visitor.visit_buffer(0, self.obj_params);
    }
}

//--------------------------------------------------------------------------------------------------
struct PipelineAndLayout<'a> {
    pipeline: &'a GraphicsPipeline,
    per_frame_descriptor_set_layout: &'a DescriptorSetLayout,
    per_object_descriptor_set_layout: &'a DescriptorSetLayout,
}

fn create_pipelines<'rcx, 'a>(arena: &'a Arena<'rcx, Backend>) -> PipelineAndLayout<'a> {
    // load pipeline file
    let pp = gl_backend::PipelineDescriptionFile::load(arena, "tests/data/shaders/deferred.glsl")
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

    let per_frame_descriptor_set_layout = arena.create_descriptor_set_layout(&[
        // camera parameters
        DescriptorSetLayoutBinding {
            binding: 0,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            descriptor_type: DescriptorType::UniformBuffer,
            count: 1,
            tydesc: None,
        },
    ]);

    let per_object_descriptor_set_layout = arena.create_descriptor_set_layout(&[
        // per-object parameters
        DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
            tydesc: None,
        },
        // diffuse texture
        DescriptorSetLayoutBinding {
            binding: 1,
            descriptor_type: DescriptorType::SampledImage,
            stage_flags: ShaderStageFlags::FRAGMENT,
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
        depth_attachment: AttachmentDescription {
            format: Format::D32_SFLOAT,
            samples: 1,
        }
        .into(),
        color_attachments: &[
            AttachmentDescription {
                format: Format::R8G8B8A8_SRGB,
                samples: 1,
            }, // albedo
            AttachmentDescription {
                format: Format::R16G16_SFLOAT,
                samples: 1,
            }, // normals
            AttachmentDescription {
                format: Format::R16G16_UINT,
                samples: 1,
            }, // object ID
            AttachmentDescription {
                format: Format::R16G16_SFLOAT,
                samples: 1,
            }, // velocity
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
        pipeline: arena.create_graphics_pipeline(&gci),
        per_frame_descriptor_set_layout,
        per_object_descriptor_set_layout,
    }
}

//#[derive(FramebufferAttachments)]
struct GBuffers<'a> {
    //#[color_attachment(0)]
    //#[format(R16G16B16A16_SFLOAT)]
    normal: &'a Image,
    //#[color_attachment(1)]
    tangents: &'a Image,
}

struct SimplePipelineInterface<'a> {
    // #[fragment_output]
    framebuffer: &'a Framebuffer,
    // #[descriptor_set(0)]
    per_frame_data: &'a DescriptorSet,
    // #[descriptor_set(1)]
    per_object_data: &'a DescriptorSet,
    // #[viewport]
    viewport: Viewport,
    // #[vertex_input(0)]
    vertex_buffer: &'a Buffer,
}

impl<'a> PipelineInterface<'a, Backend> for SimplePipelineInterface<'a> {
    // TODO
    const VERTEX_INPUT_INTERFACE: &'static [VertexInputBufferDescription<'static>] = &[];
    const FRAGMENT_OUTPUT_INTERFACE: &'static [FragmentOutputDescription] = &[];
    const DESCRIPTOR_SET_INTERFACE: &'static [DescriptorSetDescription<'static>] = &[];

    fn do_visit(&self, visitor: &mut PipelineInterfaceVisitor<'a, Backend>) {
        visitor.visit_descriptor_sets(&[self.per_frame_data, self.per_object_data]);
        visitor.visit_framebuffer(self.framebuffer);
        visitor.visit_vertex_buffers(&[self.vertex_buffer]);
    }
}

//--------------------------------------------------------------------------------------------------
fn main() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR"));

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let mut app = App::new();

    let mut first = true;
    let mut should_close = false;

    let r = app.renderer();
    let arena_long_lived = r.create_arena();
    // create pipeline
    let pipeline = create_pipelines(&arena_long_lived);
    let long_lived_buffer = arena_long_lived.create_immutable_buffer(64, &[0, 0, 0, 0, 0, 0, 0, 0]);

    // swapchain-sized resource scope
    loop {
        let default_swapchain = r.default_swapchain().unwrap();
        let (w, h) = default_swapchain.size();

        info!("Allocating swapchain resources ({}x{})", w, h);
        let arena_swapchain = r.create_arena();

        let color_buffer = arena_swapchain.create_image(
            AliasScope::no_alias(),
            Format::R16G16B16A16_SFLOAT,
            (w, h).into(),
            MipmapsCount::One,
            1,
            ImageUsageFlags::COLOR_ATTACHMENT,
        );

        let depth_buffer = arena_swapchain.create_image(
            AliasScope::no_alias(),
            Format::D32_SFLOAT,
            (w, h).into(),
            MipmapsCount::One,
            1,
            ImageUsageFlags::COLOR_ATTACHMENT,
        );

        let framebuffer = arena_swapchain.create_framebuffer(&[color_buffer], Some(depth_buffer));

        // inner event loop (frame-based resource scope)
        while !should_close {
            should_close = app.poll_events(|event| {});

            let a = r.create_arena();
            let camera_params = a.create_immutable_buffer(64, &[0, 0, 0, 0, 0, 0, 0, 0]);
            let object_params = a.create_immutable_buffer(64, &[0, 0, 0, 0, 0, 0, 0, 0]);

            let per_frame_data = a.create_descriptor_set(
                pipeline.per_frame_descriptor_set_layout,
                PerFrameUniforms {
                    camera_params,
                    test: long_lived_buffer,
                },
            );

            let per_object_data = a.create_descriptor_set(
                pipeline.per_object_descriptor_set_layout,
                PerObjectUniforms {
                    obj_params: long_lived_buffer,
                },
            );

            let mut cmdbuf = r.create_command_buffer();
            cmdbuf.clear_image(0x0, &color_buffer, &[0.0, 0.2, 0.8, 1.0]);
            cmdbuf.clear_depth_stencil_image(0x0, &depth_buffer, 1.0, None);

            cmdbuf.draw(
                0x0,
                pipeline.pipeline,
                &SimplePipelineInterface {
                    framebuffer,
                    per_frame_data,
                    per_object_data,
                    viewport: Viewport {
                        x: 0.0.into(),
                        y: 0.0.into(),
                        width: (w as f32).into(),
                        height: (h as f32).into(),
                        min_depth: 0.0.into(),
                        max_depth: 1.0.into(),
                    },
                    vertex_buffer: long_lived_buffer,
                },
            );

            /*cmdbuf.draw(PipelineInterface {
                framebuffer: a.create_framebuffer(&[color_buffer]),
            });*/

            cmdbuf.present(0x0, &color_buffer, default_swapchain);
            r.submit_frame(vec![cmdbuf]);

            // should break the loop if swapchain was resized
        }

        if should_close {
            break;
        }
    }
}
