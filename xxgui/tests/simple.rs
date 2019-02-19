#![feature(const_type_id)]
#![feature(proc_macro_hygiene)]
use autograph_render::buffer::StructuredBufferData;
use autograph_render::command::DrawIndexedParams;
use autograph_render::command::DrawParams;
use autograph_render::format::Format;
use autograph_render::glm;
use autograph_render::image::ImageUsageFlags;
use autograph_render::image::MipmapsCount;
use autograph_render::include_shader;
use autograph_render::pipeline::Arguments;
use autograph_render::pipeline::ColorBlendAttachmentState;
use autograph_render::pipeline::ColorBlendAttachments;
use autograph_render::pipeline::ColorBlendState;
use autograph_render::pipeline::DepthStencilState;
use autograph_render::pipeline::GraphicsPipelineCreateInfo;
use autograph_render::pipeline::GraphicsShaderStages;
use autograph_render::pipeline::InputAssemblyState;
use autograph_render::pipeline::MultisampleState;
use autograph_render::pipeline::PrimitiveTopology;
use autograph_render::pipeline::RasterisationState;
use autograph_render::pipeline::Scissors;
use autograph_render::pipeline::Viewport;
use autograph_render::pipeline::ViewportState;
use autograph_render::pipeline::Viewports;
use autograph_render::vertex::VertexData;
use autograph_render::AliasScope;
use autograph_render_boilerplate::*;
use log::{debug, info, warn};
use lyon::extra::rust_logo::build_logo_path;
use lyon::path::builder::SvgPathBuilder;
use lyon::path::builder::*;
use lyon::path::default::Path;
use lyon::tessellation::geometry_builder::vertex_builder;
use lyon::tessellation::geometry_builder::BuffersBuilder;
use lyon::tessellation::geometry_builder::VertexBuffers;
use lyon::tessellation::geometry_builder::VertexConstructor;
use lyon::tessellation::FillOptions;
use lyon::tessellation::FillTessellator;
use lyon::tessellation::FillVertex;
use lyon::tessellation::StrokeOptions;
use lyon::tessellation::StrokeTessellator;
use lyon::tessellation::StrokeVertex;
use std::env;

static BACKGROUND_VERT: &[u8] = include_shader!("background.vert");
static BACKGROUND_FRAG: &[u8] = include_shader!("background.frag");
static PATH_VERT: &[u8] = include_shader!("path.vert");
static PATH_FRAG: &[u8] = include_shader!("path.frag");

type Backend = autograph_render_gl::OpenGlBackend;
type Arena<'a> = autograph_render::Arena<'a, Backend>;
type Buffer<'a, T> = autograph_render::buffer::Buffer<'a, Backend, T>;
//type BufferTypeless<'a> = autograph_render::buffer::BufferTypeless<'a, Backend>;
type Image<'a> = autograph_render::image::Image<'a, Backend>;
type TypedGraphicsPipeline<'a, T> =
    autograph_render::pipeline::TypedGraphicsPipeline<'a, Backend, T>;
type TypedArgumentBlock<'a, T> = autograph_render::pipeline::TypedArgumentBlock<'a, Backend, T>;

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

#[derive(Arguments)]
#[argument(backend = "Backend")]
struct RenderTargets<'a> {
    #[argument(render_target)]
    color_target: Image<'a>,
    #[argument(viewport)]
    viewport: Viewport,
}

#[derive(Arguments)]
#[argument(backend = "Backend")]
struct Background<'a> {
    #[argument(inherit)]
    render_targets: TypedArgumentBlock<'a, RenderTargets<'a>>,
    #[argument(uniform_buffer)]
    params: Buffer<'a, BackgroundParams>,
    #[argument(vertex_buffer)]
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
    primitives: [Primitive; 32],
}

#[derive(Arguments)]
#[argument(backend = "Backend")]
struct PathRendering<'a> {
    #[argument(inherit)]
    render_targets: TypedArgumentBlock<'a, RenderTargets<'a>>,
    // set=0 binding=0
    #[argument(uniform_buffer)]
    params: Buffer<'a, BackgroundParams>,
    // set=0 binding=1
    #[argument(uniform_buffer)]
    primitives: Buffer<'a, Primitive>,
    #[argument(vertex_buffer)]
    vertex_buffer: Buffer<'a, [VertexPath]>,
    #[argument(index_buffer)]
    index_buffer: Buffer<'a, [u16]>,
}

struct Pipelines<'a> {
    background: TypedGraphicsPipeline<'a, Background<'a>>,
    path: TypedGraphicsPipeline<'a, PathRendering<'a>>,
}

fn create_pipelines<'a>(arena: &'a Arena) -> Pipelines<'a> {
    let background = GraphicsPipelineCreateInfo {
        shader_stages: arena.create_vertex_fragment_shader_stages(BACKGROUND_VERT, BACKGROUND_FRAG),
        viewport_state: ViewportState::default(),
        rasterization_state: RasterisationState::default(),
        multisample_state: MultisampleState::default(),
        depth_stencil_state: DepthStencilState::default(),
        input_assembly_state: InputAssemblyState::default(),
        color_blend_state: ColorBlendState::DISABLED,
    };

    let background = arena.create_graphics_pipeline(&background);

    let path = GraphicsPipelineCreateInfo {
        shader_stages: arena.create_vertex_fragment_shader_stages(PATH_VERT, PATH_FRAG),
        viewport_state: ViewportState::default(),
        rasterization_state: RasterisationState::default(),
        multisample_state: MultisampleState::default(),
        depth_stencil_state: DepthStencilState::default(),
        input_assembly_state: InputAssemblyState::default(),
        color_blend_state: ColorBlendState::DISABLED,
    };

    let path = arena.create_graphics_pipeline(&path);

    Pipelines { background, path }
}

//--------------------------------------------------------------------------------------------------
// Lyon test
struct TessPath<'a> {
    num_vertices: usize,
    num_indices: usize,
    vertex_buffer: Buffer<'a, [VertexPath]>,
    index_buffer: Buffer<'a, [u16]>,
}

fn tesselate_path<'a>(arena: &'a Arena) -> (TessPath<'a>, TessPath<'a>) {
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
                prim_id: 0,
            }
        }
    }
    impl VertexConstructor<FillVertex, VertexPath> for VertexCtor {
        fn new_vertex(&mut self, input: FillVertex) -> VertexPath {
            VertexPath {
                pos: [input.position.x, input.position.y],
                normal: [input.normal.x, input.normal.y],
                prim_id: 0,
            }
        }
    }

    let stroke = {
        let mut buffers: VertexBuffers<VertexPath, u16> = VertexBuffers::new();
        let mut buffers_builder = vertex_builder(&mut buffers, VertexCtor);

        StrokeTessellator::new().tessellate_path(
            path.path_iter(),
            &StrokeOptions::tolerance(tolerance).dont_apply_line_width(),
            &mut buffers_builder,
        );

        let vb = arena.upload_slice(&buffers.vertices);
        let ib = arena.upload_slice(&buffers.indices);

        TessPath {
            num_vertices: buffers.vertices.len(),
            num_indices: buffers.indices.len(),
            vertex_buffer: vb,
            index_buffer: ib,
        }
    };

    let fill = {
        let mut buffers: VertexBuffers<VertexPath, u16> = VertexBuffers::new();
        let mut buffers_builder = vertex_builder(&mut buffers, VertexCtor);

        FillTessellator::new().tessellate_path(
            path.path_iter(),
            &FillOptions::tolerance(tolerance),
            &mut buffers_builder,
        );

        let vb = arena.upload_slice(&buffers.vertices);
        let ib = arena.upload_slice(&buffers.indices);

        TessPath {
            num_vertices: buffers.vertices.len(),
            num_indices: buffers.indices.len(),
            vertex_buffer: vb,
            index_buffer: ib,
        }
    };

    (stroke, fill)
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
    let (stroke_path, fill_path) = tesselate_path(&arena_0);

    'outer: loop {
        let default_swapchain = r.default_swapchain().unwrap();
        let (w, h) = default_swapchain.size();
        let arena_1 = r.create_arena();
        let color_buffer =
            arena_1.create_unaliasable_render_target(Format::R16G16B16A16_SFLOAT, (w, h), 8);

        let render_targets = arena_1.create_typed_argument_block(RenderTargets {
            color_target: color_buffer,
            viewport: (w, h).into(),
        });

        let (left, top, right, bottom) = (-1.0, 1.0, 1.0, -1.0);

        let vertex_buffer = arena_1.upload_slice(&[
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
            let mut cmdbuf = r.create_command_buffer();
            cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);

            //----------------------------------------------------------------------------------
            // Draw background
            let background_params = arena_2.upload(&BackgroundParams {
                resolution: glm::vec2(w as f32, h as f32),
                scroll_offset: glm::vec2(0.0, 0.0),
                zoom: 5.0,
            });

            cmdbuf.draw(
                0x0,
                &arena_2,
                pipelines.background,
                Background {
                    render_targets,
                    params: background_params,
                    vertex_buffer,
                },
                DrawParams {
                    instance_count: 1,
                    first_instance: 0,
                    vertex_count: 6,
                    first_vertex: 0,
                },
            );

            //----------------------------------------------------------------------------------
            // Draw path

            // fill
            cmdbuf.draw_indexed(
                0x0,
                &arena_2,
                pipelines.path,
                PathRendering {
                    render_targets,
                    params: background_params,
                    primitives: arena_2.upload(&Primitive {
                        color: glm::vec4(1.0, 1.0, 1.0, 1.0),
                        z_index: 0,
                        width: 0.0,
                        translate: glm::vec2(-70.0, -70.0),
                    }),
                    vertex_buffer: fill_path.vertex_buffer,
                    index_buffer: fill_path.index_buffer,
                },
                DrawIndexedParams {
                    first_index: 0,
                    index_count: fill_path.num_indices as u32,
                    vertex_offset: 0,
                    first_instance: 0,
                    instance_count: 1,
                },
            );

            // stroke
            cmdbuf.draw_indexed(
                0x0,
                &arena_2,
                pipelines.path,
                PathRendering {
                    render_targets,
                    params: background_params,
                    primitives: arena_2.upload(&Primitive {
                        color: glm::vec4(0.0, 0.0, 0.0, 1.0),
                        z_index: 0,
                        width: 1.0,
                        translate: glm::vec2(-70.0, -70.0),
                    }),
                    vertex_buffer: stroke_path.vertex_buffer,
                    index_buffer: stroke_path.index_buffer,
                },
                DrawIndexedParams {
                    first_index: 0,
                    index_count: stroke_path.num_indices as u32,
                    vertex_offset: 0,
                    first_instance: 0,
                    instance_count: 1,
                },
            );

            //----------------------------------------------------------------------------------
            // Present
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
