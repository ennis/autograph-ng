extern crate gfx2;
extern crate nalgebra_glm as glm;

use std::env;

use gfx2::app::*;
use gfx2::renderer::backend::gl as gl_backend;
use gfx2::renderer::*;

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

// descriptor set layout = struct
// - descriptor::UniformBuffer<T>
// - descriptor::SampledImage
// - descriptor::Sampler
// - descriptor::StorageBuffer<T>
//
// vertex input binding = struct [repr(C)]
//
// pipeline interface = struct
// - members: descriptor set layout structs
// -
//
// derive(DescriptorSetLayout) -> reusable between different pipelines as DescriptorSetLayouts
// derive(PushConstantInterface)
// derive(FragmentOutputInterface) -> reusable?
// derive(VertexInputInterface) -> not reusable
// derive(BufferInterface)
// derive(PipelineInterface)
// derive(AttachmentGroup)
//
// trait PipelineInterface {
//      fn fragment_output_interface_description() -> &'static AttachmentDescription
//      fn fragment_output_interface(&self) -> impl Iterator<&AttachmentReference<ImageView>>
//      fn vertex_input_interface(&self) -> impl Iterator<&VertexBuffer>
// }
//
// Some interfaces can be determined from the types (but not all)
// - Descriptor set interfaces
// - Push constant interface
// - Framebuffer interface
// - Vertex input interface
// The rest must be specified explicitly (or through custom attributes?)

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

fn create_pipelines(
    renderer: &Renderer<gl_backend::OpenGlBackend>,
) -> gl_backend::GraphicsPipelineHandle {
    // load pipeline file
    let pp =
        gl_backend::PipelineDescriptionFile::load("tests/data/shaders/deferred.glsl", renderer)
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

    let per_frame_descriptor_set_layout = renderer.create_descriptor_set_layout(&[
        // camera parameters
        LayoutBinding {
            binding: 0,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            descriptor_type: DescriptorType::UniformBuffer,
            count: 1,
        },
    ]);

    let per_object_descriptor_set_layout = renderer.create_descriptor_set_layout(&[
        // per-object parameters
        LayoutBinding {
            binding: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
        },
        // diffuse texture
        LayoutBinding {
            binding: 0,
            descriptor_type: DescriptorType::SampledImage,
            stage_flags: ShaderStageFlags::FRAGMENT,
            count: 1,
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
        descriptor_map: pp.preprocessed.descriptor_map.clone(),
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

    renderer.create_graphics_pipeline(&gci)
}

//--------------------------------------------------------------------------------------------------
fn main() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR"));

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let mut app = App::new();

    let mut first = true;
    let mut should_close = false;

    let r = app.renderer();

    // create pipeline
    let pipeline = create_pipelines(r);

    while !should_close {
        should_close = app.poll_events(|event| {});

        let default_swapchain = r.default_swapchain().unwrap();
        let (w, h) = r.swapchain_dimensions(default_swapchain);

        // create resources
        // FP16 color buffer
        let color_buffer = r.create_scoped_image(
            Scope::global(),
            Format::R16G16B16A16_SFLOAT,
            (w, h).into(),
            MipmapsCount::One,
            1,
            ImageUsageFlags::COLOR_ATTACHMENT,
        );

        let depth_buffer = r.create_scoped_image(
            Scope::global(),
            Format::D32_SFLOAT,
            (w, h).into(),
            MipmapsCount::One,
            1,
            ImageUsageFlags::COLOR_ATTACHMENT,
        );

        // load pipeline
        //let pipeline = r.create_pipeline(combined_shader_source, &[gbuffers_layout, per_object_layout])

        let mut cmdbuf = r.create_command_buffer();
        cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);
        cmdbuf.clear_depth_stencil_image(0x0, depth_buffer, 1.0, None);
        cmdbuf.present(0x0, color_buffer, default_swapchain);
        r.submit_command_buffer(cmdbuf);
        r.end_frame();

        // destroy after the frame is submitted
        r.destroy_image(color_buffer);
        r.destroy_image(depth_buffer);
    }
}
