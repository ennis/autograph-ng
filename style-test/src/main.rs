#![feature(proc_macro_hygiene)]
#![feature(const_type_id)]
use autograph_render::buffer::Buffer;
use autograph_render::buffer::StructuredBufferData;
use autograph_render::descriptor::DescriptorSet;
use autograph_render::descriptor::DescriptorSetInterface;
use autograph_render::framebuffer::Framebuffer;
use autograph_render::glm;
use autograph_render::image::SampledImage;
use autograph_render::include_shader;
use autograph_render::pipeline::Arguments;
use autograph_render::pipeline::Viewport;
use autograph_render::vertex::VertexData;

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

#[derive(DescriptorSetInterface)]
pub struct PigmentApplicationDescriptorSet<'a> {
    #[descriptor(uniform_buffer)]
    pub params: Buffer<'a, PigmentApplicationParams>,
    #[descriptor(sampled_image)]
    pub filter_tex: SampledImage<'a>,
    #[descriptor(sampled_image)]
    pub substrate_tex: SampledImage<'a>,
    #[descriptor(sampled_image)]
    pub control_tex: SampledImage<'a>,
}

#[derive(DescriptorSetInterface)]
pub struct EdgeDetectionDescriptorSet<'a> {
    #[descriptor(sampled_image)]
    pub depth_tex: SampledImage<'a>,
}

#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
pub struct GradientEdgesWatercolorParams {
    substrate_color: glm::Vec3,
    edge_intensity: f32,
}

#[derive(DescriptorSetInterface)]
pub struct GradientEdgesWatercolorDescriptorSet<'a> {
    #[descriptor(uniform_buffer)]
    params: Buffer<'a, GradientEdgesWatercolorParams>,
    #[descriptor(sampled_image)]
    edge_tex_sampler: SampledImage<'a>,
    #[descriptor(sampled_image)]
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

#[derive(DescriptorSetInterface)]
pub struct SubstrateDescriptorSet<'a> {
    #[descriptor(uniform_buffer)]
    pub params: Buffer<'a, SubstrateParams>,
    #[descriptor(sampled_image)]
    pub substrate_tex: SampledImage<'a>,
    #[descriptor(sampled_image)]
    pub edge_tex: SampledImage<'a>,
    #[descriptor(sampled_image)]
    pub control_tex: SampledImage<'a>,
    #[descriptor(sampled_image)]
    pub depth_tex: SampledImage<'a>,
}

#[derive(Arguments)]
pub struct SubstrateCommon<'a> {
    #[pipeline(framebuffer)]
    pub framebuffer: Framebuffer<'a>,
    #[pipeline(descriptor_set)]
    pub substrate: DescriptorSet<'a, SubstrateDescriptorSet<'a>>,
    #[pipeline(viewport)]
    pub viewport: Viewport,
}

/// Pipeline interface for substrate shaders.
#[derive(Arguments)]
pub struct SubstrateSimple<'a> {
    #[pipeline(inherit)]
    pub common: SubstrateCommon<'a>,
    #[pipeline(vertex_buffer)]
    pub vertex_buffer: Buffer<'a, [Vertex]>,
}

/// Pipeline interface for substrate shaders that need UV coordinates
#[derive(Arguments)]
pub struct SubstrateUV<'a> {
    #[pipeline(inherit)]
    pub common: SubstrateCommon<'a>,
    #[pipeline(vertex_buffer)]
    pub vertex_buffer: Buffer<'a, [VertexUV]>,
}

fn main() {
    // load multi-channel EXR with control targets
    // - normals
    // - depth
    // - control targets
    // load substrate
    //
}
