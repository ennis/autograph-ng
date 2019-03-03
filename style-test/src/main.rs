#![feature(proc_macro_hygiene)]
use autograph_render::{
    buffer::StructuredBufferData,
    command::DrawParams,
    format::Format,
    glm,
    image::{ImageUsageFlags, MipmapsCount, SamplerDescription},
    include_shader,
    pipeline::{
        Arguments, ColorBlendAttachmentState, ColorBlendAttachments, ColorBlendState,
        DepthStencilState, GraphicsPipelineCreateInfo, GraphicsShaderStages, InputAssemblyState,
        MultisampleState, PrimitiveTopology, RasterisationState, Viewport, ViewportState,
        Viewports,
    },
    vertex::VertexData,
    AliasScope,
};
//use autograph_render_extra;
use autograph_imgui::ImGuiRenderer;
use autograph_render::buffer::BoolU32;
use autograph_render_boilerplate::{App, Event, KeyboardInput, WindowEvent};
use autograph_render_extra::{commandext::CommandBufferExt, quad::Quad};
use autograph_render_gl::OpenGlBackend;
use imgui::{im_str, FontGlyphRange, ImGui};
use log::{debug, info, warn};
use openimageio as oiio;
use std::{env, iter, mem, path::Path, slice, time};

// Define shorthands for backend-specific types ----------------------------------------------------
type Backend = autograph_render_gl::OpenGlBackend;
type Arena<'a> = autograph_render::Arena<'a, Backend>;
type Buffer<'a, T> = autograph_render::buffer::Buffer<'a, Backend, T>;
//type BufferTypeless<'a> = autograph_render::buffer::BufferTypeless<'a, Backend>;
type Image<'a> = autograph_render::image::Image<'a, Backend>;
type TextureImageView<'a> = autograph_render::image::TextureImageView<'a, Backend>;
type TypedGraphicsPipeline<'a, T> =
    autograph_render::pipeline::TypedGraphicsPipeline<'a, Backend, T>;
type TypedArgumentBlock<'a, T> = autograph_render::pipeline::TypedArgumentBlock<'a, Backend, T>;

// Shaders -----------------------------------------------------------------------------------------
static QUAD_VERT: &[u8] = include_shader!("quad.vert");
static QUAD_SAMPLER_VERT: &[u8] = include_shader!("quadSampler.vert");
static PIGMENT_APPLICATION_OIL_PAINT_FRAG: &[u8] = include_shader!("pigmentApplicationOP.frag");
static PIGMENT_APPLICATION_WATERCOLOR_FRAG: &[u8] = include_shader!("pigmentApplicationWC.frag");
static SUBSTRATE_DEFERRED_LIGHTING_FRAG: &[u8] = include_shader!("substrateDeferredLighting.frag");
static SUBSTRATE_DEFERRED_IMPASTO_LIGHTING_FRAG: &[u8] =
    include_shader!("substrateDeferredImpastoLighting.frag");
static SUBSTRATE_DISTORTION_FRAG: &[u8] = include_shader!("substrateDistortion.frag");
static SUBSTRATE_DISTORTION_EDGES_FRAG: &[u8] = include_shader!("substrateDistortionEdges.frag");
static GRADIENT_EDGES_WATERCOLOR_FRAG: &[u8] = include_shader!("gradientEdgesWC.frag");
static EDGE_DETECTION_SOBEL_RGBD_FRAG: &[u8] = include_shader!("edgeDetectionSobelRGBD.frag");
static EDGE_DETECTION_DOG_RGBD_FRAG: &[u8] = include_shader!("edgeDetectionDoGRGBD.frag");
static WATERCOLOR_FILTER_H_FRAG: &[u8] = include_shader!("watercolorFilterH.frag");
static WATERCOLOR_FILTER_V_FRAG: &[u8] = include_shader!("watercolorFilterV.frag");

static WATERCOLOR_SHADING_VERT: &[u8] = include_shader!("watercolorShading.vert");
static WATERCOLOR_SHADING_FRAG: &[u8] = include_shader!("watercolorShading.frag");

#[derive(VertexData, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    position: glm::Vec3,
}

#[derive(VertexData, Copy, Clone)]
#[repr(C)]
pub struct Vertex2D {
    position: glm::Vec2,
}

impl Vertex2D {
    pub fn new(pos: [f32; 2]) -> Vertex2D {
        Vertex2D {
            position: glm::vec2(pos[0], pos[1]),
        }
    }
}

#[derive(VertexData, Copy, Clone)]
#[repr(C)]
pub struct Vertex2DTexcoord {
    position: glm::Vec2,
    texcoord: glm::Vec2,
}

impl Vertex2DTexcoord {
    pub fn new(pos: [f32; 2], texcoord: [f32; 2]) -> Vertex2DTexcoord {
        Vertex2DTexcoord {
            position: glm::vec2(pos[0], pos[1]),
            texcoord: glm::vec2(texcoord[0], texcoord[1]),
        }
    }
}

#[derive(VertexData, Copy, Clone)]
#[repr(C)]
pub struct VertexUV {
    position: glm::Vec3,
    uv: glm::Vec2,
}
/*
#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
pub struct SubstrateParams {
    gamma : f32,
    substrate_light_dir : f32,
    substrate_light_tilt : f32,
    substrate_shading : f32,
    substrate_distortion: f32,
    impasto_phong_specular : f32,
    impasto_phong_shininess : f32,
}
*/

#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
pub struct PigmentApplicationParams {
    substrate_color: glm::Vec3,
    pigment_density: f32,
    dry_brush_threshold: f32,
}

impl Default for PigmentApplicationParams {
    fn default() -> Self {
        PigmentApplicationParams {
            substrate_color: glm::vec3(1.0, 1.0, 1.0),
            pigment_density: 1.0,
            dry_brush_threshold: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, StructuredBufferData)]
pub struct CommonUniforms {
    wvp: glm::Mat4,
    screen_size: glm::Vec2,
    _padding: [f32; 2],
    luminance_coeff: glm::Vec3,
}

#[derive(Arguments, Copy, Clone)]
#[argument(backend = "Backend")]
pub struct CommonArguments<'a> {
    #[argument(uniform_buffer)]
    pub uniforms: Buffer<'a, CommonUniforms>,
    #[argument(sampled_image)]
    pub color_tex: TextureImageView<'a>,
    #[argument(viewport)]
    pub viewport: Viewport,
    //#[argument(vertex_buffer)]
    //pub quad: Buffer<'a, [Vertex2DTexcoord]>,
}

#[derive(Arguments, Copy, Clone)]
#[argument(backend = "Backend")]
pub struct PigmentApplication<'a> {
    #[argument(inherit)]
    pub common: TypedArgumentBlock<'a, CommonArguments<'a>>,
    #[argument(uniform_buffer)]
    pub params: Buffer<'a, PigmentApplicationParams>,
    #[argument(sampled_image)]
    pub filter_tex: TextureImageView<'a>,
    #[argument(sampled_image)]
    pub substrate_tex: TextureImageView<'a>,
    #[argument(sampled_image)]
    pub control_tex: TextureImageView<'a>,
}

#[derive(Arguments, Copy, Clone)]
#[argument(backend = "Backend")]
pub struct EdgeDetection<'a> {
    #[argument(inherit)]
    pub common: TypedArgumentBlock<'a, CommonArguments<'a>>,
    #[argument(render_target)]
    pub edge_out: Image<'a>,
    #[argument(sampled_image)]
    pub depth_tex: TextureImageView<'a>,
}

//type EdgeDetectionParams<'a> = Quad<'a,OpenGlBackend,EdgeDetection<'a>>;

#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
pub struct GradientEdgesWatercolorParams {
    substrate_color: glm::Vec3,
    edge_intensity: f32,
}

#[derive(Arguments, Copy, Clone)]
#[argument(backend = "Backend")]
pub struct GradientEdgesWatercolor<'a> {
    #[argument(uniform_buffer)]
    params: Buffer<'a, GradientEdgesWatercolorParams>,
    #[argument(sampled_image)]
    edge_tex_sampler: TextureImageView<'a>,
    #[argument(sampled_image)]
    control_tex_sampler: TextureImageView<'a>,
}

#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
pub struct SubstrateParams {
    gamma: f32,
    substrate_light_dir: f32,
    substrate_light_tilt: f32,
    substrate_shading: f32,
    substrate_distortion: f32,
    impasto_phong_specular: f32,
    impasto_phong_shininess: f32,
}

impl Default for SubstrateParams {
    fn default() -> Self {
        SubstrateParams {
            gamma: 1.0,
            substrate_light_dir: 0.0,
            substrate_light_tilt: 45.0,
            substrate_shading: 1.0,
            substrate_distortion: 1.0,
            impasto_phong_specular: 0.6,
            impasto_phong_shininess: 16.0,
        }
    }
}

#[derive(Arguments, Copy, Clone)]
#[argument(backend = "Backend")]
pub struct SubstrateCommon<'a> {
    #[argument(inherit)]
    pub common: TypedArgumentBlock<'a, CommonArguments<'a>>,
    #[argument(render_target)]
    pub color_target: Image<'a>,
    #[argument(uniform_buffer)]
    pub params: Buffer<'a, SubstrateParams>,
    #[argument(sampled_image)]
    pub substrate_tex: TextureImageView<'a>,
    #[argument(sampled_image)]
    pub edge_tex: TextureImageView<'a>,
    #[argument(sampled_image)]
    pub control_tex: TextureImageView<'a>,
    #[argument(sampled_image)]
    pub depth_tex: TextureImageView<'a>,
}

#[repr(C)]
#[derive(StructuredBufferData, Copy, Clone)]
pub struct WatercolorShadingParams {
    color_tint: glm::Vec3,
    _padding0: u32,
    shade_color: glm::Vec3,
    _padding1: u32,
    paper_color: glm::Vec3,
    _padding2: u32,
    g_screen_size: glm::Vec2, // screen size, in pixels
    use_control: BoolU32,     // < string UIWidget = "None"; > = true
    use_color_texture: BoolU32,
    use_normal_texture: BoolU32,
    flip_u: BoolU32,
    flip_v: BoolU32,
    bump_depth: f32,
    use_specular_texture: BoolU32,
    specular: f32,
    spec_diffusion: f32,
    spec_transparency: f32,
    use_shadows: BoolU32,
    shadow_depth_bias: f32,
    diffuse_factor: f32,
    shade_wrap: f32,
    use_override_shade: BoolU32,
    dilute: f32,
    cangiante: f32,
    dilute_area: f32,
    high_area: f32,
    high_transparency: f32,
    dark_edges: f32,
    tremor: f32,
    tremor_front: f32,
    tremor_speed: f32,
    tremor_freq: f32,
    bleed_offset: f32,
}

impl Default for WatercolorShadingParams {
    fn default() -> Self {
        WatercolorShadingParams {
            g_screen_size: glm::vec2(640.0, 480.0),
            _padding0: 0,
            _padding1: 0,
            _padding2: 0,
            use_control: BoolU32::False,
            use_color_texture: BoolU32::False,
            color_tint: glm::vec3(1.0, 1.0, 1.0),
            use_normal_texture: BoolU32::False,
            flip_u: BoolU32::False,
            flip_v: BoolU32::False,
            bump_depth: 1.0,
            use_specular_texture: BoolU32::False,
            specular: 0.0,
            spec_diffusion: 0.0,
            spec_transparency: 0.0,
            use_shadows: BoolU32::False,
            shadow_depth_bias: 0.001,
            diffuse_factor: 0.2,
            shade_color: glm::vec3(0.0, 0.0, 0.0),
            shade_wrap: 0.0,
            use_override_shade: BoolU32::True,
            dilute: 0.8,
            cangiante: 0.2,
            dilute_area: 1.0,
            high_area: 0.0,
            high_transparency: 0.0,
            dark_edges: 0.0,
            tremor: 4.0,
            tremor_front: 0.4,
            tremor_speed: 10.0,
            tremor_freq: 10.0,
            paper_color: glm::vec3(1.0, 1.0, 1.0),
            bleed_offset: 0.5,
        }
    }
}

#[derive(Arguments, Copy, Clone)]
#[argument(backend = "Backend")]
struct WatercolorShading<'a> {
    #[argument(render_target)]
    target: Image<'a>,
    #[argument(viewport)]
    viewport: Viewport,
    #[argument(uniform_buffer)]
    params: Buffer<'a, WatercolorShadingParams>,
    #[argument(sampled_image)]
    diffuse_color: TextureImageView<'a>,
    #[argument(sampled_image)]
    diffuse_direct_lighting: TextureImageView<'a>,
    #[argument(sampled_image)]
    specular_color: TextureImageView<'a>,
    #[argument(sampled_image)]
    specular_direct_lighting: TextureImageView<'a>,
    #[argument(sampled_image)]
    ambient_occlusion: TextureImageView<'a>,
}

struct Pipelines<'a> {
    edge_detection_dog_rgbd: TypedGraphicsPipeline<'a, Quad<'a, OpenGlBackend, EdgeDetection<'a>>>,
    edge_detection_sobel_rgbd:
        TypedGraphicsPipeline<'a, Quad<'a, OpenGlBackend, EdgeDetection<'a>>>,
    substrate_deferred_lighting:
        TypedGraphicsPipeline<'a, Quad<'a, OpenGlBackend, SubstrateCommon<'a>>>,
    watercolor_shading: TypedGraphicsPipeline<'a, Quad<'a, OpenGlBackend, WatercolorShading<'a>>>,
    substrate_distortion:
        TypedGraphicsPipeline<'a, Quad<'a, OpenGlBackend, SubstrateCommon<'a>>>,
}

impl<'a> Pipelines<'a> {
    pub fn create(arena: &'a Arena) -> Pipelines<'a> {
        let edge_detection_dog_rgbd = GraphicsPipelineCreateInfo {
            shader_stages: arena.create_vertex_fragment_shader_stages(
                QUAD_SAMPLER_VERT,
                EDGE_DETECTION_DOG_RGBD_FRAG,
            ),
            viewport_state: ViewportState::default(),
            rasterization_state: RasterisationState::default(),
            multisample_state: MultisampleState::default(),
            depth_stencil_state: DepthStencilState::default(),
            input_assembly_state: InputAssemblyState::default(),
            color_blend_state: ColorBlendState::DISABLED,
        };

        let edge_detection_sobel_rgbd = GraphicsPipelineCreateInfo {
            shader_stages: arena.create_vertex_fragment_shader_stages(
                QUAD_SAMPLER_VERT,
                EDGE_DETECTION_SOBEL_RGBD_FRAG,
            ),
            viewport_state: ViewportState::default(),
            rasterization_state: RasterisationState::default(),
            multisample_state: MultisampleState::default(),
            depth_stencil_state: DepthStencilState::default(),
            input_assembly_state: InputAssemblyState::default(),
            color_blend_state: ColorBlendState::DISABLED,
        };

        let substrate_deferred_lighting = GraphicsPipelineCreateInfo {
            shader_stages: arena.create_vertex_fragment_shader_stages(
                QUAD_SAMPLER_VERT,
                SUBSTRATE_DEFERRED_LIGHTING_FRAG,
            ),
            viewport_state: ViewportState::default(),
            rasterization_state: RasterisationState::default(),
            multisample_state: MultisampleState::default(),
            depth_stencil_state: DepthStencilState::default(),
            input_assembly_state: InputAssemblyState::default(),
            color_blend_state: ColorBlendState::DISABLED,
        };

        let watercolor_shading = GraphicsPipelineCreateInfo {
            shader_stages: arena.create_vertex_fragment_shader_stages(
                WATERCOLOR_SHADING_VERT,
                WATERCOLOR_SHADING_FRAG,
            ),
            viewport_state: ViewportState::default(),
            rasterization_state: RasterisationState::default(),
            multisample_state: MultisampleState::default(),
            depth_stencil_state: DepthStencilState::default(),
            input_assembly_state: InputAssemblyState::default(),
            color_blend_state: ColorBlendState::DISABLED,
        };

        let substrate_distortion = GraphicsPipelineCreateInfo {
            shader_stages: arena.create_vertex_fragment_shader_stages(
                QUAD_SAMPLER_VERT,
                SUBSTRATE_DISTORTION_FRAG,
            ),
            viewport_state: ViewportState::default(),
            rasterization_state: RasterisationState::default(),
            multisample_state: MultisampleState::default(),
            depth_stencil_state: DepthStencilState::default(),
            input_assembly_state: InputAssemblyState::default(),
            color_blend_state: ColorBlendState::DISABLED,
        };

        Pipelines {
            edge_detection_dog_rgbd: arena.create_graphics_pipeline(&edge_detection_dog_rgbd),
            edge_detection_sobel_rgbd: arena.create_graphics_pipeline(&edge_detection_sobel_rgbd),
            substrate_deferred_lighting: arena
                .create_graphics_pipeline(&substrate_deferred_lighting),
            watercolor_shading: arena.create_graphics_pipeline(&watercolor_shading),
            substrate_distortion: arena.create_graphics_pipeline(&substrate_distortion),
        }
    }
}

const DIFFUSE_COLOR_CHANNEL_NAMES: &[&str] = &[
    "RenderLayer.DiffCol.R",
    "RenderLayer.DiffCol.G",
    "RenderLayer.DiffCol.B",
];

const DIFFUSE_DIRECT_COMPONENT_CHANNEL_NAMES: &[&str] = &[
    "RenderLayer.DiffDir.R",
    "RenderLayer.DiffDir.G",
    "RenderLayer.DiffDir.B",
];

const SPECULAR_COLOR_CHANNEL_NAMES: &[&str] = &[
    "RenderLayer.GlossCol.R",
    "RenderLayer.GlossCol.G",
    "RenderLayer.GlossCol.B",
];

const SPECULAR_DIRECT_COMPONENT_CHANNEL_NAMES: &[&str] = &[
    "RenderLayer.GlossDir.R",
    "RenderLayer.GlossDir.G",
    "RenderLayer.GlossDir.B",
];

const NORMAL_CHANNEL_NAMES: &[&str] = &[
    "RenderLayer.Normal.Y",
    "RenderLayer.Normal.Z",
    "RenderLayer.Normal.X",
];

const DEPTH_CHANNEL_NAME: &[&str] = &["RenderLayer.Depth.Z"];

const AO_CHANNEL_NAMES: &[&str] = &[
    "RenderLayer.AO.R",
    //"RenderLayer.AO.G",
    //"RenderLayer.AO.B",
];

fn load_image_data<'a, T: oiio::ImageData>(
    a: &'a Arena,
    input: &mut oiio::ImageInput,
    chans: &[&str],
    fmt: Format,
) -> Image<'a> {
    let (w, h, _) = input.spec().size();
    let data: oiio::ImageBuffer<T> = input.channels_by_name(chans).unwrap().read().unwrap();

    a.create_immutable_image(
        fmt,
        (w, h).into(),
        MipmapsCount::One,
        1,
        ImageUsageFlags::SAMPLED,
        data.as_bytes(),
    )
}

const FONT: &[u8] = include_bytes!("../../imgui/tests/ChiKareGo2.ttf");
const FONT_SIZE: f32 = 15.0;

pub struct ImGuiContext {
    app_hidpi_factor: f64,
    imgui: imgui::ImGui,
    last_frame_time: time::Instant,
}

impl ImGuiContext {
    pub fn new(app_hidpi_factor: f64) -> ImGuiContext {
        let mut imgui = imgui::ImGui::init();
        imgui
            .fonts()
            .add_font(FONT, FONT_SIZE, &FontGlyphRange::default());
        imgui_winit_support::configure_keys(&mut imgui);
        ImGuiContext {
            app_hidpi_factor,
            imgui,
            last_frame_time: time::Instant::now(),
        }
    }

    pub fn handle_event(&mut self, window: &winit::Window, event: &winit::Event) {
        imgui_winit_support::handle_event(
            &mut self.imgui,
            event,
            window.get_hidpi_factor(),
            self.app_hidpi_factor,
        );
    }

    pub fn frame(&mut self, window: &winit::Window) -> imgui::Ui {
        let frame_size =
            imgui_winit_support::get_frame_size(window, self.app_hidpi_factor).unwrap();
        let elapsed = self.last_frame_time.elapsed();
        let delta_time =
            (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1_000_000_000.0);
        self.last_frame_time = time::Instant::now();
        self.imgui.frame(frame_size, delta_time as f32)
    }

    pub fn imgui(&mut self) -> &mut ImGui {
        &mut self.imgui
    }
}

fn main() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let app = App::new();
    let r = app.renderer();

    let arena_0 = r.create_arena();
    let pipelines = Pipelines::create(&arena_0);

    // load test image
    let mut img = oiio::ImageInput::open("data/output0013.exr").unwrap();

    let (frame_width, frame_height, _) = img.spec().size();

    let diffuse_color = load_image_data::<u16>(
        &arena_0,
        &mut img,
        DIFFUSE_COLOR_CHANNEL_NAMES,
        Format::R16G16B16_UNORM,
    );
    let diffuse_direct_lighting = load_image_data::<u16>(
        &arena_0,
        &mut img,
        DIFFUSE_DIRECT_COMPONENT_CHANNEL_NAMES,
        Format::R16G16B16_UNORM,
    );
    let specular_color = load_image_data::<u16>(
        &arena_0,
        &mut img,
        SPECULAR_COLOR_CHANNEL_NAMES,
        Format::R16G16B16_UNORM,
    );
    let specular_direct_lighting = load_image_data::<u16>(
        &arena_0,
        &mut img,
        SPECULAR_DIRECT_COMPONENT_CHANNEL_NAMES,
        Format::R16G16B16_UNORM,
    );
    let normals = load_image_data::<f32>(
        &arena_0,
        &mut img,
        NORMAL_CHANNEL_NAMES,
        Format::R32G32B32_SFLOAT,
    );
    let ambient_occlusion =
        load_image_data::<u16>(&arena_0, &mut img, AO_CHANNEL_NAMES, Format::R16_UNORM);
    let depth = load_image_data::<f32>(&arena_0, &mut img, DEPTH_CHANNEL_NAME, Format::R32_SFLOAT);

    // load substrate texture
    let mut substrate_img = oiio::ImageInput::open("data/rough_default_2k.jpg").unwrap();
    let substrate_data: oiio::ImageBuffer<u16> = substrate_img.all_channels().read().unwrap();
    let substrate = arena_0.create_immutable_image(
        Format::R16G16B16_UNORM,
        (
            substrate_data.width() as u32,
            substrate_data.height() as u32,
        )
            .into(),
        MipmapsCount::One,
        1,
        ImageUsageFlags::SAMPLED,
        substrate_data.as_bytes(),
    );

    // control map
    let control = arena_0.create_image(
        AliasScope::no_alias(),
        Format::R8G8B8A8_UNORM,
        (frame_width, frame_height).into(),
        MipmapsCount::One,
        1,
        ImageUsageFlags::SAMPLED,
    );

    // edge map
    let edge_map = arena_0.create_image(
        AliasScope::no_alias(),
        Format::R32_SFLOAT,
        (frame_width, frame_height).into(),
        MipmapsCount::One,
        1,
        ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
    );

    // lighting
    let lighting = arena_0.create_image(
        AliasScope::no_alias(),
        Format::R16G16B16_UNORM,
        (frame_width, frame_height).into(),
        MipmapsCount::One,
        1,
        ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
    );

    let color_buffer_2 = arena_0.create_image(
        AliasScope::no_alias(),
        Format::R16G16B16_UNORM,
        (frame_width, frame_height).into(),
        MipmapsCount::One,
        1,
        ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
    );

    // clear control map
    let mut cmdbuf = r.create_command_buffer();
    cmdbuf.clear_image(0x0, control, &[0.0, 0.0, 0.0, 0.0]);
    r.submit_frame(iter::once(cmdbuf));

    // create imgui context
    let mut imguictx = ImGuiContext::new(1.0);

    let mut watercolor_shading_params = WatercolorShadingParams {
        ..Default::default()
    };
    let mut substrate_params = SubstrateParams {
        ..Default::default()
    };

    'outer: loop {
        let default_swapchain = r.default_swapchain().unwrap();
        let (w, h) = default_swapchain.size();
        let arena_1 = r.create_arena();
        let color_buffer =
            arena_1.create_unaliasable_render_target(Format::R16G16B16A16_SFLOAT, (w, h), 8);

        // UI renderer
        let mut imgui_renderer =
            ImGuiRenderer::new(&arena_1, imguictx.imgui(), color_buffer, (w, h).into());

        'inner: loop {
            //----------------------------------------------------------------------------------
            // handle events
            let should_close = app.poll_events(|event| imguictx.handle_event(app.window(), &event));
            let arena_frame = r.create_arena();

            let mut cmdbuf = r.create_command_buffer();

            // Clear background
            cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);

            // common arguments
            let common = arena_frame.create_typed_argument_block(CommonArguments {
                uniforms: arena_frame.upload(&CommonUniforms {
                    wvp: glm::identity(),
                    screen_size: glm::vec2(w as f32, h as f32),
                    _padding: [0.0; 2],
                    luminance_coeff: glm::vec3(1.0, 1.0, 1.0),
                }),
                color_tex: lighting.into_texture_view_linear(),
                viewport: (frame_width, frame_height).into(),
            });

            let common2 = arena_frame.create_typed_argument_block(CommonArguments {
                uniforms: arena_frame.upload(&CommonUniforms {
                    wvp: glm::identity(),
                    screen_size: glm::vec2(w as f32, h as f32),
                    _padding: [0.0; 2],
                    luminance_coeff: glm::vec3(1.0, 1.0, 1.0),
                }),
                color_tex: color_buffer_2.into_texture_view_linear(),
                viewport: (frame_width, frame_height).into(),
            });


           /* //----------------------------------------------------------------------------------
            // Run edge detection
            cmdbuf.draw_quad(
                0x0,
                &arena_frame,
                pipelines.edge_detection_dog_rgbd,
                EdgeDetection {
                    common,
                    edge_out: edge_map,
                    depth_tex: depth.into_texture_view_linear(),
                },
            );*/

            //----------------------------------------------------------------------------------
            // Run shading
            cmdbuf.draw_quad(
                0x0,
                &arena_frame,
                pipelines.watercolor_shading,
                WatercolorShading {
                    target: lighting,
                    viewport: (frame_width, frame_height).into(),
                    params: arena_frame.upload(&watercolor_shading_params),
                    diffuse_color: diffuse_color.into_texture_view_linear(),
                    diffuse_direct_lighting: diffuse_direct_lighting.into_texture_view_linear(),
                    specular_color: specular_color.into_texture_view_linear(),
                    specular_direct_lighting: specular_direct_lighting.into_texture_view_linear(),
                    ambient_occlusion: ambient_occlusion.into_texture_view_linear(),
                },
            );


            //----------------------------------------------------------------------------------
            // Run distortion
            cmdbuf.draw_quad(
                0x0,
                &arena_frame,
                pipelines.substrate_distortion,
                SubstrateCommon {
                    common,
                    color_target: color_buffer_2,
                    params: arena_frame.upload(&substrate_params),
                    substrate_tex: substrate.into_texture_view_linear(),
                    edge_tex: edge_map.into_texture_view_linear(),
                    control_tex: control.into_texture_view_linear(),
                    depth_tex: depth.into_texture_view_linear(),
                },
            );


            //----------------------------------------------------------------------------------
            // Run substrate shading
            cmdbuf.draw_quad(
                0x0,
                &arena_frame,
                pipelines.substrate_deferred_lighting,
                SubstrateCommon {
                    common: common2,
                    color_target: color_buffer,
                    params: arena_frame.upload(&substrate_params),
                    substrate_tex: substrate.into_texture_view_linear(),
                    edge_tex: edge_map.into_texture_view_linear(),
                    control_tex: control.into_texture_view_linear(),
                    depth_tex: depth.into_texture_view_linear(),
                },
            );

            //----------------------------------------------------------------------------------
            // Parameter UI
            let mut ui = imguictx.frame(app.window());
            let mut open = true;
            ui.show_demo_window(&mut open);
            ui.slider_float(
                im_str!("Cangiante"),
                &mut watercolor_shading_params.cangiante,
                0.0,
                1.0,
            )
            .build();
            ui.slider_float(
                im_str!("Dilute"),
                &mut watercolor_shading_params.dilute,
                0.0,
                1.0,
            )
            .build();

            ui.slider_float(im_str!("gamma"), &mut substrate_params.gamma, 0.0, 1.0).build(); // 1.0,
            ui.slider_float(im_str!("substrate_light_dir"), &mut substrate_params.substrate_light_dir, 0.0, 90.0).build(); // 0.0,
            ui.slider_float(im_str!("substrate_light_tilt"), &mut substrate_params.substrate_light_tilt, 0.0, 90.0).build(); // 45.0,
            ui.slider_float(im_str!("substrate_shading"), &mut substrate_params.substrate_shading, 0.0, 1.0).build(); // 1.0,
            ui.slider_float(im_str!("substrate_distortion"), &mut substrate_params.substrate_distortion, 0.0, 30.0).build(); // 1.0,
            ui.slider_float(im_str!("impasto_phong_specular"), &mut substrate_params.impasto_phong_specular, 0.0, 1.0).build(); // 0.6,
            ui.slider_float(im_str!("impasto_phong_shininess"), &mut substrate_params.impasto_phong_shininess, 0.0, 100.0).build(); // 16.0,

            ui.color_picker(
                im_str!("Paper color"),
                watercolor_shading_params.paper_color.as_mut(),
            )
            .build();
            //ui.color_picker(im_str!("Paper color"), watercolor_shading_params.);
            imgui_renderer.render(&mut cmdbuf, 0x0, &arena_frame, ui);

            //----------------------------------------------------------------------------------
            // Present edge map
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
