#![feature(proc_macro_hygiene)]
use autograph_render::{
    buffer::StructuredBufferData,
    command::{DrawIndexedParams, DrawParams},
    format::Format,
    glm, include_shader,
    pipeline::{
        Arguments, ColorBlendState, DepthStencilState, GraphicsPipelineCreateInfo,
        InputAssemblyState, MultisampleState, RasterisationState, Viewport, ViewportState,
    },
    vertex::VertexData,
};
use autograph_render_test::*;
use lyon::{
    extra::rust_logo::build_logo_path,
    path::{
        builder::{SvgPathBuilder, *},
        default::Path,
    },
    tessellation::{
        geometry_builder::{vertex_builder, VertexBuffers, VertexConstructor},
        FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator, StrokeVertex,
    },
};
use std::iter;

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

#[derive(Arguments)]
#[argument(backend = "Backend")]
struct RenderTargets<'a> {
    #[argument(render_target)]
    color_target: RenderTargetView<'a>,
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
    _num_vertices: usize,
    num_indices: usize,
    vertex_buffer: Buffer<'a, [VertexPath]>,
    index_buffer: Buffer<'a, [u16]>,
}

fn tessellate_path<'a>(arena: &'a Arena) -> (TessPath<'a>, TessPath<'a>) {
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
            _num_vertices: buffers.vertices.len(),
            num_indices: buffers.indices.len(),
            vertex_buffer: vb,
            index_buffer: ib,
        }
    };

    let fill = {
        let mut buffers: VertexBuffers<VertexPath, u16> = VertexBuffers::new();
        let mut buffers_builder = vertex_builder(&mut buffers, VertexCtor);

        FillTessellator::new()
            .tessellate_path(
                path.path_iter(),
                &FillOptions::tolerance(tolerance),
                &mut buffers_builder,
            )
            .unwrap();

        let vb = arena.upload_slice(&buffers.vertices);
        let ib = arena.upload_slice(&buffers.indices);

        TessPath {
            _num_vertices: buffers.vertices.len(),
            num_indices: buffers.indices.len(),
            vertex_buffer: vb,
            index_buffer: ib,
        }
    };

    (stroke, fill)
}

fn main() {
    autograph_render_test::with_test_fixture(
        "lyon tessellation test",
        None,
        |renderer, arena, main_loop| {
            let pipelines = create_pipelines(arena);
            let (stroke_path, fill_path) = tessellate_path(arena);
            let default_swapchain = renderer.default_swapchain().unwrap();
            let (w, h) = default_swapchain.size();
            let color_buffer =
                arena.create_unaliasable_render_target(Format::R16G16B16A16_SFLOAT, (w, h), 8);

            let render_targets = arena.create_typed_argument_block(RenderTargets {
                color_target: color_buffer.into(),
                viewport: (w, h).into(),
            });

            let (left, top, right, bottom) = (-1.0, 1.0, 1.0, -1.0);

            let vertex_buffer = arena.upload_slice(&[
                Vertex2D::new([left, top]),
                Vertex2D::new([right, top]),
                Vertex2D::new([left, bottom]),
                Vertex2D::new([left, bottom]),
                Vertex2D::new([right, top]),
                Vertex2D::new([right, bottom]),
            ]);

            main_loop.run(|| {
                let arena = renderer.create_arena();
                let mut cmdbuf = renderer.create_command_buffer();

                //----------------------------------------------------------------------------------
                // Draw background
                let background_params = arena.upload(&BackgroundParams {
                    resolution: glm::vec2(w as f32, h as f32),
                    scroll_offset: glm::vec2(0.0, 0.0),
                    zoom: 5.0,
                });

                cmdbuf.draw(
                    0x0,
                    &arena,
                    pipelines.background,
                    Background {
                        render_targets,
                        params: background_params,
                        vertex_buffer,
                    },
                    DrawParams::quad(),
                );

                //----------------------------------------------------------------------------------
                // Draw path

                // fill
                cmdbuf.draw_indexed(
                    0x0,
                    &arena,
                    pipelines.path,
                    PathRendering {
                        render_targets,
                        params: background_params,
                        primitives: arena.upload(&Primitive {
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
                    &arena,
                    pipelines.path,
                    PathRendering {
                        render_targets,
                        params: background_params,
                        primitives: arena.upload(&Primitive {
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
                renderer.submit_frame(iter::once(cmdbuf));
            })
        },
    );
}
