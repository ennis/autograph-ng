#![feature(proc_macro_hygiene)]
use autograph_render::buffer::StructuredBufferData;
use autograph_render::command::DrawParams;
use autograph_render::format::Format;
use autograph_render::glm;
use autograph_render::image::ImageUsageFlags;
use autograph_render::image::MipmapsCount;
use autograph_render::image::SamplerDescription;
use autograph_render::include_shader;
use autograph_render::pipeline::Arguments;
use autograph_render::pipeline::ColorBlendAttachmentState;
use autograph_render::pipeline::ColorBlendAttachments;
use autograph_render::pipeline::ColorBlendState;
use autograph_render::pipeline::DepthStencilState;
use autograph_render::pipeline::DynamicStateFlags;
use autograph_render::pipeline::GraphicsPipelineCreateInfo;
use autograph_render::pipeline::GraphicsShaderStages;
use autograph_render::pipeline::InputAssemblyState;
use autograph_render::pipeline::MultisampleState;
use autograph_render::pipeline::PrimitiveTopology;
use autograph_render::pipeline::RasterisationState;
use autograph_render::pipeline::Scissors;
use autograph_render::pipeline::ShaderStageFlags;
use autograph_render::pipeline::Viewport;
use autograph_render::pipeline::ViewportState;
use autograph_render::pipeline::Viewports;
use autograph_render::vertex::VertexData;
use autograph_render::AliasScope;
use autograph_render_boilerplate::App;
use autograph_render_boilerplate::Event;
use autograph_render_boilerplate::KeyboardInput;
use autograph_render_boilerplate::WindowEvent;
use log::{debug, info, warn};
use openimageio as oiio;
use std::env;
use std::mem;
use std::slice;

type Backend = autograph_render_gl::OpenGlBackend;
type Arena<'a> = autograph_render::Arena<'a, Backend>;
type Buffer<'a, T> = autograph_render::buffer::Buffer<'a, Backend, T>;
//type BufferTypeless<'a> = autograph_render::buffer::BufferTypeless<'a, Backend>;
type Image<'a> = autograph_render::image::Image<'a, Backend>;
type SampledImage<'a> = autograph_render::image::SampledImage<'a, Backend>;
type TypedGraphicsPipeline<'a, T> =
    autograph_render::pipeline::TypedGraphicsPipeline<'a, Backend, T>;
type TypedArgumentBlock<'a, T> = autograph_render::pipeline::TypedArgumentBlock<'a, Backend, T>;

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
    pub color_tex: SampledImage<'a>,
    #[argument(viewport)]
    pub viewport: Viewport,
    #[argument(vertex_buffer)]
    pub quad: Buffer<'a, [Vertex2DTexcoord]>,
}

#[derive(Arguments, Copy, Clone)]
#[argument(backend = "Backend")]
pub struct PigmentApplication<'a> {
    #[argument(inherit)]
    pub common: TypedArgumentBlock<'a, CommonArguments<'a>>,
    #[argument(uniform_buffer)]
    pub params: Buffer<'a, PigmentApplicationParams>,
    #[argument(sampled_image)]
    pub filter_tex: SampledImage<'a>,
    #[argument(sampled_image)]
    pub substrate_tex: SampledImage<'a>,
    #[argument(sampled_image)]
    pub control_tex: SampledImage<'a>,
}

#[derive(Arguments, Copy, Clone)]
#[argument(backend = "Backend")]
pub struct EdgeDetection<'a> {
    #[argument(inherit)]
    pub common: TypedArgumentBlock<'a, CommonArguments<'a>>,
    #[argument(render_target)]
    pub edge_out: Image<'a>,
    #[argument(sampled_image)]
    pub depth_tex: SampledImage<'a>,
}

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
    edge_tex_sampler: SampledImage<'a>,
    #[argument(sampled_image)]
    control_tex_sampler: SampledImage<'a>,
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
    #[argument(render_target)]
    pub color_target: Image<'a>,
    #[argument(viewport)]
    pub viewport: Viewport,
    #[argument(uniform_buffer)]
    pub params: Buffer<'a, SubstrateParams>,
    #[argument(sampled_image)]
    pub substrate_tex: SampledImage<'a>,
    #[argument(sampled_image)]
    pub edge_tex: SampledImage<'a>,
    #[argument(sampled_image)]
    pub control_tex: SampledImage<'a>,
    #[argument(sampled_image)]
    pub depth_tex: SampledImage<'a>,
    #[argument(vertex_buffer)]
    pub vertex_buffer: Buffer<'a, [Vertex2DTexcoord]>,
}

struct Pipelines<'a> {
    edge_detection_dog_rgbd: TypedGraphicsPipeline<'a, EdgeDetection<'a>>,
    edge_detection_sobel_rgbd: TypedGraphicsPipeline<'a, EdgeDetection<'a>>,
}

impl<'a> Pipelines<'a> {
    pub fn create(a: &'a Arena) -> Pipelines<'a> {
        let edge_detection_dog_rgbd = GraphicsPipelineCreateInfo {
            shader_stages: &GraphicsShaderStages {
                vertex: a.create_shader_module(QUAD_SAMPLER_VERT, ShaderStageFlags::VERTEX),
                geometry: None,
                fragment: Some(a.create_shader_module(
                    EDGE_DETECTION_DOG_RGBD_FRAG,
                    ShaderStageFlags::FRAGMENT,
                )),
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
            dynamic_state: DynamicStateFlags::empty(),
        };

        let edge_detection_sobel_rgbd = GraphicsPipelineCreateInfo {
            shader_stages: &GraphicsShaderStages {
                vertex: a.create_shader_module(QUAD_SAMPLER_VERT, ShaderStageFlags::VERTEX),
                geometry: None,
                fragment: Some(a.create_shader_module(
                    EDGE_DETECTION_SOBEL_RGBD_FRAG,
                    ShaderStageFlags::FRAGMENT,
                )),
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
            dynamic_state: DynamicStateFlags::empty(),
        };

        Pipelines {
            edge_detection_dog_rgbd: a.create_graphics_pipeline(&edge_detection_dog_rgbd),
            edge_detection_sobel_rgbd: a.create_graphics_pipeline(&edge_detection_sobel_rgbd),
        }
    }
}

fn main() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let app = App::new();
    let r = app.renderer();

    let arena_0 = r.create_arena();
    let pipelines = Pipelines::create(&arena_0);

    'outer: loop {
        let default_swapchain = r.default_swapchain().unwrap();
        let (w, h) = default_swapchain.size();

        let arena_1 = r.create_arena();

        let color_buffer = arena_1.create_image(
            AliasScope::no_alias(),
            Format::R16G16B16A16_SFLOAT,
            (w, h).into(),
            MipmapsCount::One,
            8, // multisampling
            ImageUsageFlags::COLOR_ATTACHMENT,
        );

        let (left, top, right, bottom) = (-1.0, 1.0, 1.0, -1.0);

        let quad = arena_0.upload_slice(&[
            Vertex2DTexcoord::new([left, top], [0.0, 1.0]),
            Vertex2DTexcoord::new([right, top], [1.0, 1.0]),
            Vertex2DTexcoord::new([left, bottom], [0.0, 0.0]),
            Vertex2DTexcoord::new([left, bottom], [0.0, 0.0]),
            Vertex2DTexcoord::new([right, top], [1.0, 1.0]),
            Vertex2DTexcoord::new([right, bottom], [1.0, 0.0]),
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

            let arena_frame = r.create_arena();

            let mut cmdbuf = r.create_command_buffer();

            // Clear background
            cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);

            // load next frame of animation
            let normals = arena_frame.create_image(
                AliasScope::no_alias(),
                Format::R16G16_SFLOAT,
                (w, h).into(),
                MipmapsCount::One,
                1,
                ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
            );

            let mut img =
                oiio::ImageInput::open("../openimageio-rs/test_images/output0013.exr").unwrap();
            let subimage_0 = img.subimage_0();
            let (w, h) = (subimage_0.width(), subimage_0.height());

            let color_data: oiio::ImageBuffer<f32> =
                subimage_0.read(r"RenderLayer\.Combined\.[RGBA]").unwrap();
            assert_eq!(color_data.num_channels(), 4);

            let normal_data: oiio::ImageBuffer<f32> =
                subimage_0.read(r"RenderLayer\.Normal\.[XYZ]").unwrap();
            assert_eq!(normal_data.num_channels(), 3);

            let depth_data: oiio::ImageBuffer<f32> =
                subimage_0.read(r"RenderLayer\.Depth\.Z").unwrap();
            assert_eq!(depth_data.num_channels(), 1);

            let depth = arena_frame.create_immutable_image(
                Format::R32_SFLOAT,
                (w, h).into(),
                MipmapsCount::One,
                1,
                ImageUsageFlags::DEPTH_ATTACHMENT | ImageUsageFlags::SAMPLED,
                depth_data.as_bytes(),
            );

            let diffuse = arena_frame.create_immutable_image(
                Format::R32G32B32A32_SFLOAT,
                (w, h).into(),
                MipmapsCount::One,
                1,
                ImageUsageFlags::DEPTH_ATTACHMENT | ImageUsageFlags::SAMPLED,
                color_data.as_bytes(),
            );

            // TODO transfer normals, depth, base color from file

            // edge map
            let edge_map = arena_frame.create_image(
                AliasScope::no_alias(),
                Format::R32_SFLOAT,
                (w, h).into(),
                MipmapsCount::One,
                1,
                ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
            );

            // common arguments
            let common = arena_frame.create_typed_argument_block(CommonArguments {
                uniforms: arena_frame.upload(&CommonUniforms {
                    wvp: glm::identity(),
                    screen_size: glm::vec2(w as f32, h as f32),
                    _padding: [0.0; 2],
                    luminance_coeff: glm::vec3(1.0, 1.0, 1.0),
                }),
                color_tex: color_buffer.into_sampled(SamplerDescription::LINEAR_MIPMAP_LINEAR),
                viewport: (w, h).into(),
                quad,
            });

            //----------------------------------------------------------------------------------
            // Run edge detection
            cmdbuf.draw(
                0x0,
                &arena_frame,
                pipelines.edge_detection_dog_rgbd,
                EdgeDetection {
                    common,
                    edge_out: edge_map,
                    depth_tex: depth.into_sampled(SamplerDescription::LINEAR_MIPMAP_LINEAR),
                },
                DrawParams {
                    instance_count: 1,
                    first_instance: 0,
                    vertex_count: 6,
                    first_vertex: 0,
                },
            );

            //----------------------------------------------------------------------------------
            // Present edge map
            cmdbuf.present(0x0, edge_map, default_swapchain);
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
