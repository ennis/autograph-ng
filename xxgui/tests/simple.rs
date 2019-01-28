#![feature(const_type_id)]
#![feature(proc_macro_hygiene)]
use autograph_render::buffer::Buffer;
use autograph_render::buffer::StructuredBufferData;
use autograph_render::command::DrawParams;
use autograph_render::descriptor::DescriptorSetInterface;
use autograph_render::format::Format;
use autograph_render::framebuffer::Framebuffer;
use autograph_render::glm;
use autograph_render::image::ImageUsageFlags;
use autograph_render::image::MipmapsCount;
use autograph_render::image::SampledImage;
use autograph_render::include_shader;
use autograph_render::pipeline::ColorBlendAttachmentState;
use autograph_render::pipeline::ColorBlendAttachments;
use autograph_render::pipeline::ColorBlendState;
use autograph_render::pipeline::DepthStencilState;
use autograph_render::pipeline::GraphicsPipeline;
use autograph_render::pipeline::GraphicsPipelineCreateInfo;
use autograph_render::pipeline::GraphicsShaderStages;
use autograph_render::pipeline::InputAssemblyState;
use autograph_render::pipeline::MultisampleState;
use autograph_render::pipeline::PipelineInterface;
use autograph_render::pipeline::PipelineLayout;
use autograph_render::pipeline::PrimitiveTopology;
use autograph_render::pipeline::RasterisationState;
use autograph_render::pipeline::Scissors;
use autograph_render::pipeline::ShaderStageFlags;
use autograph_render::pipeline::Viewport;
use autograph_render::pipeline::ViewportState;
use autograph_render::pipeline::Viewports;
use autograph_render::vertex::VertexData;
use autograph_render::AliasScope;
use autograph_render::Arena;
use autograph_render_boilerplate::*;
use log::{debug, info, warn};
use std::env;
use lyon::path::default::Path;
use lyon::path::builder::SvgPathBuilder;
use lyon::extra::rust_logo::build_logo_path;
use lyon::path::builder::*;
use lyon::tessellation::geometry_builder::VertexBuffers;
use lyon::tessellation::geometry_builder::BuffersBuilder;
use lyon::tessellation::geometry_builder::vertex_builder;
use lyon::tessellation::geometry_builder::VertexConstructor;
use lyon::tessellation::FillTessellator;
use lyon::tessellation::StrokeVertex;
use lyon::tessellation::FillOptions;
use lyon::tessellation::StrokeTessellator;
use lyon::tessellation::StrokeOptions;

static BACKGROUND_VERT: &[u8] = include_shader!("background.vert");
static BACKGROUND_FRAG: &[u8] = include_shader!("background.frag");
static PATH_VERT: &[u8] = include_shader!("path.vert");
static PATH_FRAG: &[u8] = include_shader!("path.frag");

//--------------------------------------------------------------------------------------------------
// Shader stuff

#[derive(Copy, Clone, VertexData)]
#[repr(C)]
pub struct Vertex2D {
    pub pos: [f32; 2],
    //pub tex: [f32; 2],
}

impl Vertex2D {
    pub fn new(pos: [f32; 2]) -> Vertex2D {
        Vertex2D { pos }
    }
}

#[derive(Copy, Clone, VertexData)]
#[repr(C)]
pub struct VertexPath {
    pub pos: [f32; 2],
    pub normal: [f32; 2],
    pub prim_id: i32,
}

impl VertexPath {
    pub fn new(pos: [f32; 2], normal: [f32; 2], prim_id: i32) -> VertexPath {
        VertexPath {
            pos,
            normal,
            prim_id,
        }
    }
}

#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
struct BackgroundParams {
    resolution: glm::Vec2,
    scroll_offset: glm::Vec2,
    zoom: f32,
}

#[derive(DescriptorSetInterface, Copy, Clone)]
struct BackgroundDescriptorSet<'a> {
    #[descriptor(uniform_buffer)]
    params: Buffer<'a, BackgroundParams>,
}

#[derive(PipelineInterface)]
struct BackgroundPipeline<'a> {
    #[pipeline(framebuffer)]
    framebuffer: Framebuffer<'a>,
    #[pipeline(descriptor_set)]
    descriptor_set: BackgroundDescriptorSet<'a>,
    #[pipeline(viewport)]
    viewport: Viewport,
    #[pipeline(vertex_buffer)]
    vertex_buffer: Buffer<'a, [Vertex2D]>,
}

#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
struct Primitive {
    color: glm::Vec4,
    z_index: i32,
    width: f32,
    translate: glm::Vec2,
}

#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
struct PrimitiveArray {
    primitives: [Primitive; 32]
}


#[derive(DescriptorSetInterface, Copy, Clone)]
struct PathDescriptorSet<'a> {
    #[descriptor(uniform_buffer)]
    params: Buffer<'a, BackgroundParams>,
    #[descriptor(uniform_buffer)]
    primitives: Buffer<'a, PrimitiveArray>,
}


#[derive(PipelineInterface)]
struct PathPipeline<'a> {
    #[pipeline(framebuffer)]
    framebuffer: Framebuffer<'a>,
    #[pipeline(descriptor_set)]
    descriptor_set: PathDescriptorSet<'a>,
    #[pipeline(viewport)]
    viewport: Viewport,
    #[pipeline(vertex_buffer)]
    vertex_buffer: Buffer<'a, [Vertex2D]>,
}

struct Pipelines<'a> {
    background: GraphicsPipeline<'a, BackgroundPipeline<'a>>,
    path: GraphicsPipeline<'a, PathPipeline<'a>>,
}

fn create_pipelines<'a>(arena: &'a Arena) -> Pipelines<'a> {
    let background = GraphicsPipelineCreateInfo {
        shader_stages: &GraphicsShaderStages {
            vertex: arena.create_shader_module(BACKGROUND_VERT, ShaderStageFlags::VERTEX),
            geometry: None,
            fragment: Some(arena.create_shader_module(BACKGROUND_FRAG, ShaderStageFlags::FRAGMENT)),
            tess_eval: None,
            tess_control: None,
        },
        viewport_state: &ViewportState {
            viewports: Viewports::Dynamic,
            scissors: Scissors::Dynamic,
        },
        rasterization_state: &RasterisationState::DEFAULT,
        multisample_state: &MultisampleState::default(),
        depth_stencil_state: &DepthStencilState::default(),
        input_assembly_state: &InputAssemblyState {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart_enable: false,
        },
        color_blend_state: &ColorBlendState {
            attachments: ColorBlendAttachments::All(&ColorBlendAttachmentState::DISABLED),
            blend_constants: [0.0.into(); 4],
            logic_op: None,
        },
    };

    let background = arena.create_graphics_pipeline(&background, &PipelineLayout::default());

    let path = GraphicsPipelineCreateInfo {
        shader_stages: &GraphicsShaderStages {
            vertex: arena.create_shader_module(PATH_VERT, ShaderStageFlags::VERTEX),
            geometry: None,
            fragment: Some(arena.create_shader_module(PATH_FRAG, ShaderStageFlags::FRAGMENT)),
            tess_eval: None,
            tess_control: None,
        },
        viewport_state: &ViewportState {
            viewports: Viewports::Dynamic,
            scissors: Scissors::Dynamic,
        },
        rasterization_state: &RasterisationState::DEFAULT,
        multisample_state: &MultisampleState::default(),
        depth_stencil_state: &DepthStencilState::default(),
        input_assembly_state: &InputAssemblyState {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart_enable: false,
        },
        color_blend_state: &ColorBlendState {
            attachments: ColorBlendAttachments::All(&ColorBlendAttachmentState::DISABLED),
            blend_constants: [0.0.into(); 4],
            logic_op: None,
        },
    };

    let path = arena.create_graphics_pipeline(&path, &PipelineLayout::default());

    Pipelines { background, path }
}

//--------------------------------------------------------------------------------------------------
// Lyon test
fn rasterize_path<'a>(arena: &'a Arena, pipeline: GraphicsPipeline<'a, PathPipeline<'a>>)
{
    let mut builder = SvgPathBuilder::new(Path::builder());
    build_logo_path(&mut builder);
    let path = builder.build();

    let tolerance = 0.01;

    // tesselate path
    struct VertexCtor;
    impl VertexConstructor<StrokeVertex, VertexPath> for VertexCtor {
        fn new_vertex(&mut self, input: StrokeVertex) -> VertexPath {
            VertexPath {
                pos: [input.position.x, input.position.y],
                normal: [input.normal.x, input.normal.y],
                prim_id: 0
            }
        }
    }

    let mut buffers : VertexBuffers<VertexPath, u16> = VertexBuffers::new();
    let mut buffers_builder = vertex_builder(&mut buffers, VertexCtor);

    StrokeTessellator::new().tessellate_path(
        path.path_iter(),
        &StrokeOptions::tolerance(tolerance).dont_apply_line_width(),
        &mut BuffersBuilder::new(&mut buffers, VertexCtor)
    );

}

#[test]
fn test_simple() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let app = App::new();
    let r = app.renderer();

    let arena_0 = r.create_arena();
    // pipelines
    let pipelines = create_pipelines(&arena_0);

    'outer: loop {
        let default_swapchain = r.default_swapchain().unwrap();
        let (w, h) = default_swapchain.size();
        let arena_1 = r.create_arena();

        let color_buffer = arena_1.create_image(
            AliasScope::no_alias(),
            Format::R16G16B16A16_SFLOAT,
            (w, h).into(),
            MipmapsCount::One,
            1,
            ImageUsageFlags::COLOR_ATTACHMENT,
        );

        let framebuffer = arena_1.create_framebuffer(&[color_buffer], None);

        let (left, top, right, bottom) = (-1.0, 1.0, 1.0, -1.0);

        let vertex_buffer = arena_0.upload_slice(&[
            Vertex2D::new([left, top]),
            Vertex2D::new([right, top]),
            Vertex2D::new([left, bottom]),
            Vertex2D::new([left, bottom]),
            Vertex2D::new([right, top]),
            Vertex2D::new([right, bottom]),
        ]);

        'inner: loop {
            //----------------------------------------------------------------------------------
            // handle events
            let should_close = app.poll_events(|event| match event {
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(vkey),
                                    modifiers: mods,
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    info!("key={:?} mod={:?}", vkey, mods);
                }
                _ => {}
            });

            let arena_2 = r.create_arena();

            //----------------------------------------------------------------------------------
            // Draw background
            let background_params = arena_2.upload(&BackgroundParams {
                resolution: glm::vec2(w as f32, h as f32),
                scroll_offset: glm::vec2(0.0, 0.0),
                zoom: 1.0,
            });

            let background_descriptor_set = BackgroundDescriptorSet {
                params: background_params,
            };

            let mut cmdbuf = r.create_command_buffer();
            cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);

            cmdbuf.draw(
                0x0,
                &arena_2,
                pipelines.background,
                &BackgroundPipeline {
                    framebuffer,
                    descriptor_set: background_descriptor_set,
                    viewport: (w, h).into(),
                    vertex_buffer,
                },
                DrawParams {
                    instance_count: 1,
                    first_instance: 0,
                    vertex_count: 6,
                    first_vertex: 0,
                },
            );

            cmdbuf.present(0x0, color_buffer, default_swapchain);
            r.submit_frame(vec![cmdbuf]);

            if should_close {
                break 'outer;
            }

            let (new_w, new_h) = default_swapchain.size();
            // don't resize if new size is null in one dimension, as it will
            // cause create_framebuffer to fail.
            if (new_w, new_h) != (w, h) && new_w != 0 && new_h != 0 {
                break 'inner;
            }
        }
    }
}
