#[macro_use]
extern crate log;

mod common;

use self::common::*;
use autograph_render;
use autograph_render::glm;
use autograph_render::interface::DescriptorSetInterface;
use autograph_render::interface::DescriptorSetInterfaceVisitor;
use autograph_render::interface::FragmentOutputDescription;
use autograph_render::interface::PipelineInterface;
use autograph_render::interface::PipelineInterfaceVisitor;
use autograph_render::interface::VertexLayout;
use autograph_render::*;
use autograph_render_gl as gl_backend;
use std::env;
use std::mem;

type Backend = gl_backend::OpenGlBackend;
type Buffer<'a, T> = autograph_render::Buffer<'a, Backend, T>;
type BufferTypeless<'a> = autograph_render::BufferTypeless<'a, Backend>;
type Image<'a> = autograph_render::Image<'a, Backend>;
type Framebuffer<'a> = autograph_render::Framebuffer<'a, Backend>;
type DescriptorSet<'a> = autograph_render::DescriptorSetTypeless<'a, Backend>;
type DescriptorSetLayout<'a> = autograph_render::DescriptorSetLayout<'a, Backend>;
type GraphicsPipeline<'a> = autograph_render::GraphicsPipeline<'a, Backend>;

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

#[derive(Copy, Clone)]
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

#[derive(Copy, Clone)]
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

*/

//--------------------------------------------------------------------------------------------------
struct PerFrameUniforms<'a, R: RendererBackend> {
    camera_params: autograph_render::Buffer<'a, R, CameraParameters>,
    test: autograph_render::BufferTypeless<'a, R>,
}

// SHOULD BE AUTOMATICALLY DERIVED
impl<'a, R: RendererBackend> DescriptorSetInterface<'a, R> for PerFrameUniforms<'a, R> {
    const INTERFACE: &'static [DescriptorSetLayoutBinding<'static>] =
        &[DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
            tydesc: None,
        }];

    fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>) {
        visitor.visit_buffer(
            0,
            self.camera_params.into(),
            0,
            mem::size_of::<CameraParameters>(),
        );
        //visitor.visit_buffer(1, self.camera_params, 0, 64);
    }
}

//--------------------------------------------------------------------------------------------------
struct PerObjectUniforms<'a, R: RendererBackend> {
    obj_params: autograph_render::Buffer<'a, R, ObjectParameters>,
}

// SHOULD BE AUTOMATICALLY DERIVED
impl<'a, R: RendererBackend> DescriptorSetInterface<'a, R> for PerObjectUniforms<'a, R> {
    const INTERFACE: &'static [DescriptorSetLayoutBinding<'static>] =
        &[DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
            tydesc: None,
        }];

    fn do_visit(&self, visitor: &mut impl DescriptorSetInterfaceVisitor<'a, R>) {
        visitor.visit_buffer(
            0,
            self.obj_params.into(),
            0,
            mem::size_of::<ObjectParameters>(),
        );
    }
}

//--------------------------------------------------------------------------------------------------
// SHADERS & PIPELINES
autograph_render::include_combined_shader! {DeferredShaders, "tests/data/shaders/deferred.glsl", vertex, fragment}

struct PipelineAndLayout<'a> {
    pipeline: GraphicsPipeline<'a>,
    per_frame_descriptor_set_layout: DescriptorSetLayout<'a>,
    per_object_descriptor_set_layout: DescriptorSetLayout<'a>,
}

fn create_pipelines<'rcx, 'a>(arena: &'a Arena<'rcx, Backend>) -> PipelineAndLayout<'a> {
    let shader_stages = GraphicsShaderStages {
        vertex: arena.create_shader_module(DeferredShaders::VERTEX, ShaderStageFlags::VERTEX),
        geometry: None,
        fragment: Some(
            arena.create_shader_module(DeferredShaders::FRAGMENT, ShaderStageFlags::FRAGMENT),
        ),
        tess_eval: None,
        tess_control: None,
    };

    let vertex_input_state = VertexInputState {
        bindings: &[VertexInputBindingDescription {
            binding: 0,
            stride: 44,
            input_rate: VertexInputRate::Vertex,
        }],
        attributes: &[
            VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: 12,
            },
            VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: 24,
            },
            VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: Format::R32G32_SFLOAT,
                offset: 36,
            },
        ],
    };

    let viewport_state = ViewportState {
        viewports: Viewports::Dynamic,
        scissors: Scissors::Dynamic,
    };

    let rasterization_state = RasterisationState::DEFAULT;
    let depth_stencil_state = DepthStencilState::default();
    let color_blend_state = ColorBlendState {
        attachments: ColorBlendAttachments::All(&ColorBlendAttachmentState::DISABLED),
        blend_constants: [0.0.into(); 4],
        logic_op: None,
    };

    let multisample_state = MultisampleState::default();

    let input_assembly_state = InputAssemblyState {
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

    let pipeline_layout = PipelineLayout {
        descriptor_set_layouts: descriptor_set_layouts.as_ref(),
    };

    let attachment_layout = AttachmentLayout {
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
    normal: Image<'a>,
    //#[color_attachment(1)]
    tangents: Image<'a>,
}

struct SimplePipelineInterface<'a> {
    // #[fragment_output]
    framebuffer: Framebuffer<'a>,
    // #[descriptor_set(0)]
    per_frame_data: DescriptorSet<'a>,
    // #[descriptor_set(1)]
    per_object_data: DescriptorSet<'a>,
    // #[viewport]
    viewport: Viewport,
    // #[vertex_input(0)]
    vertex_buffer: BufferTypeless<'a>,
}

impl<'a> PipelineInterface<'a, Backend> for SimplePipelineInterface<'a> {
    // TODO
    const VERTEX_INPUT_INTERFACE: &'static [VertexLayout<'static>] = &[];
    const FRAGMENT_OUTPUT_INTERFACE: &'static [FragmentOutputDescription] = &[];
    const DESCRIPTOR_SET_INTERFACE: &'static [&'static [DescriptorSetLayoutBinding<'static>]] = &[];

    fn do_visit(&self, visitor: &mut PipelineInterfaceVisitor<'a, Backend>) {
        visitor.visit_descriptor_sets(&[self.per_frame_data, self.per_object_data]);
        visitor.visit_framebuffer(self.framebuffer);
        visitor.visit_vertex_buffers(&[self.vertex_buffer]);
    }
}

//--------------------------------------------------------------------------------------------------
fn main() {
    env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let mut app = App::new();
    let mut should_close = false;

    let r = app.renderer();
    let arena_long_lived = r.create_arena();
    let long_lived_buffer = arena_long_lived
        .upload_slice(&[0, 0, 0, 0, 0, 0, 0, 0])
        .into();

    // graphics pipelines
    'outer: loop {
        let arena_pipelines = r.create_arena();
        // reload pipelines
        let pipeline = create_pipelines(&arena_long_lived);

        // swapchain-sized resource scope
        'swapchain: loop {
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

            let framebuffer =
                arena_swapchain.create_framebuffer(&[color_buffer], Some(depth_buffer));

            // inner event loop (frame-based resource scope)
            'events: while !should_close {
                should_close = app.poll_events(|event| {});

                let a = r.create_arena();
                let camera_params = a.upload(&unsafe { mem::uninitialized() });
                let obj_params = a.upload(&unsafe { mem::uninitialized() });

                let per_frame_data = a.create_descriptor_set(
                    pipeline.per_frame_descriptor_set_layout,
                    PerFrameUniforms {
                        camera_params,
                        test: long_lived_buffer,
                    },
                );

                let per_object_data = a.create_descriptor_set(
                    pipeline.per_object_descriptor_set_layout,
                    PerObjectUniforms { obj_params },
                );

                let mut cmdbuf = r.create_command_buffer();
                cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);
                cmdbuf.clear_depth_stencil_image(0x0, depth_buffer, 1.0, None);

                cmdbuf.draw(
                    0x0,
                    pipeline.pipeline,
                    &SimplePipelineInterface {
                        framebuffer,
                        per_frame_data,
                        per_object_data,
                        viewport: (w, h).into(),
                        vertex_buffer: long_lived_buffer,
                    },
                    DrawParams {
                        instance_count: 1,
                        first_instance: 0,
                        vertex_count: 6,
                        first_vertex: 0,
                    },
                );

                /*cmdbuf.draw(PipelineInterface {
                    framebuffer: a.create_framebuffer(&[color_buffer]),
                });*/

                cmdbuf.present(0x0, color_buffer, default_swapchain);
                r.submit_frame(vec![cmdbuf]);
            }

            if should_close {
                break 'outer;
            }
        }
    }
}
