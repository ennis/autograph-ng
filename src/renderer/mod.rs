//! Renderer manifesto:
//!
//! - Easy to use
//! - Flexible
//! - Not too verbose
//! - Dynamic
//!
//! Based on global command reordering with sort keys.
//! (see https://ourmachinery.com/post/a-modern-rendering-architecture/)
//! Submission order is fully independant from the execution
//! order on the GPU. The necessary barriers and synchronization
//! is determined once the list of commands is sorted.
//! Thus, adding a post-proc effect is as easy as adding a command buffer with the correct resource
//! names and sequence IDs so that it happens after main rendering.
//! This means that any component in the engine can modify
//! the render pipeline 'non-locally' by submitting a command buffer.
//! This might not be a good thing per se, but at least it's flexible.
//!
//! The `Renderer` instances should be usable across threads
//! (e.g. can allocate and upload from different threads at once).
//!
//! `CommandBuffers` are renderer-agnostic.
//! They contain commands with a sort key that indicates their relative execution order.
//!
//! Unsolved questions: properly handle the sort keys.
//!
//! Idea: named resources (resource semantics)
//! - A way to assign arbitrary IDs to resources that has meaning to the application
//!     - e.g. the temporary render layer of mesh group X for appearance Y
//!     - 64-bit handles
//! - Pre-described and allocated on first use
//! - Resource templates: ID + mask that describes how to allocate a resource with the specified bit pattern
//!     - registered in advance (pipeline config)
//! - advantage: no need to pass resource handles around, just refer to them by semantics (convention over configuration)
//! - would allow draw calls like:
//! ```
//!     draw (
//!         sort_key = sequence_id!{ opaque, layer=group_id, depth=d, pass_immediate=0 },
//!         target   = stylized_layer(objgroup)
//!     )
//! ```
//! To test in the high-level renderer:
//! - 2D texturing (ss splats anchored on submeshes)
//! - Splat-based shading (splat proxies in screen space masked, directed by projected light direction)
//! - Good contour detection (explicit crease edges + fast DF)
//! - Good cast shadows (soft & hard, raycasting and temporal integration?)
//! - Real depth-sorted transparency everywhere (assume non-intersecting objects)
//! - Per-object-group screen-space calculations (e.g. curvature, surface descriptors, etc.)
//!       - Discretize into coarse grid to evaluate filter only where needed
//! - Performance might be abysmal, but that's not an issue
//!     - for our purposes, 5 fps is good enough! -> target animation
//! - Stroke-based rendering with arbitrary curves (curve DF)
//!     - big unknown, prepare for poor performance
//! - High-quality ambient occlusion?
//!
//! UI is important!
//!
//! Renderer backend: object-safe or compile-time?
//! - avoid costly recompilation times -> object-safe

use ordered_float::NotNan;
use std::cmp::Eq;
use std::fmt::Debug;
use std::mem;
use std::sync::Mutex;

pub mod backend;
mod buffer;
mod command_buffer;
mod format;
mod image;
mod sampler;
mod shader_interface;
mod sync;
mod util;

/*
define_sort_key! {
    [sequence:3  , layer:8, depth:16, pass_immediate:4],
    [opaque:3 = 3, layer:8, depth:16, pass_immediate:4],
    [shadow:3 = 1, view: 6, layer:8, depth:16, pass_immediate:4]

    sequence,objgroup,comp-pass(pre,draw,post),effect,effect-pass(pre,draw,post)
}

sequence_id!{ opaque, layer=group_id, depth=d, pass_immediate=0 }*/

pub use self::command_buffer::{sort_command_buffers, Command, CommandBuffer, CommandInner};
pub use self::format::*;
pub use self::image::*;
pub use self::sampler::*;
pub use self::shader_interface::*;

//--------------------------------------------------------------------------------------------------

/*// hackish way to have a const NotNan.
union TransmuteNotNan {
    from: f32,
    to: NotNan<f32>,
}

const unsafe fn const_notnan(v: f32) -> NotNan<f32> {
    TransmuteNotNan { from: v }.to
}*/

#[derive(Copy, Clone, Debug)]
pub enum MemoryType {
    DeviceLocal,
    HostUpload,
    HostReadback,
}

pub enum Queue {
    Graphics,
    Compute,
    Transfer,
}

bitflags! {
    #[derive(Default)]
    pub struct ShaderStageFlags: u32 {
        const VERTEX = (1 << 0);
        const GEOMETRY = (1 << 1);
        const FRAGMENT = (1 << 2);
        const TESS_CONTROL = (1 << 3);
        const TESS_EVAL = (1 << 4);
        const COMPUTE = (1 << 5);
        const ALL_GRAPHICS = Self::VERTEX.bits | Self::GEOMETRY.bits | Self::FRAGMENT.bits | Self::TESS_CONTROL.bits | Self::TESS_EVAL.bits;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VertexInputAttributeDescription {
    pub location: u32,
    pub binding: u32,
    pub format: Format,
    pub offset: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    TriangleList,
}

#[derive(Copy, Clone, Debug)]
pub struct DescriptorSetLayoutBinding<'tcx> {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub stage_flags: ShaderStageFlags,
    pub count: usize,
    pub tydesc: Option<&'tcx TypeDesc<'tcx>>,
}

#[derive(Copy, Clone, Debug)]
pub enum DescriptorType {
    Sampler, // TODO
    SampledImage,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
    InputAttachment,
}

pub enum Descriptor<'a, R: RendererBackend> {
    SampledImage {
        img: &'a R::Image,
        sampler: SamplerDescription,
    },
    Image {
        img: &'a R::Image,
    },
    Buffer {
        buffer: &'a R::Buffer,
        offset: usize,
        size: usize,
    },
    Empty,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ShaderFormat {
    SpirV,
    BackendSpecific,
}

#[derive(Debug)]
pub struct GraphicsPipelineShaderStages<'rcx, R: RendererBackend> {
    //pub format: ShaderFormat,
    pub vertex: &'rcx R::ShaderModule,
    pub geometry: Option<&'rcx R::ShaderModule>,
    pub fragment: Option<&'rcx R::ShaderModule>,
    pub tess_eval: Option<&'rcx R::ShaderModule>,
    pub tess_control: Option<&'rcx R::ShaderModule>,
}

// rust issue #26925
impl<'a, R: RendererBackend> Clone for GraphicsPipelineShaderStages<'a, R> {
    fn clone(&self) -> Self {
        // safe because TODO
        unsafe { mem::transmute_copy(self) }
    }
}

bitflags! {
    #[derive(Default)]
    pub struct CullModeFlags: u32 {
        const NONE = 0;
        const FRONT = 1;
        const BACK = 2;
        const FRONT_AND_BACK = Self::FRONT.bits | Self::BACK.bits;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct DynamicStateFlags: u32 {
        const VIEWPORT = (1 << 0);
        const SCISSOR = (1 << 1);
        const LINE_WIDTH = (1 << 2);
        const DEPTH_BIAS = (1 << 3);
        const BLEND_CONSTANTS = (1 << 4);
        const DEPTH_BOUNDS = (1 << 5);
        const STENCIL_COMPARE_MASK = (1 << 6);
        const STENCIL_WRITE_MASK = (1 << 7);
        const STENCIL_REFERENCE = (1 << 8);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PolygonMode {
    Line,
    Fill,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum FrontFace {
    Clockwise,
    CounterClockwise,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum DepthBias {
    Disabled,
    Enabled {
        constant_factor: NotNan<f32>,
        clamp: NotNan<f32>,
        slope_factor: NotNan<f32>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PipelineRasterizationStateCreateInfo {
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: PolygonMode,
    pub cull_mode: CullModeFlags,
    pub depth_bias: DepthBias,
    pub front_face: FrontFace,
    pub line_width: NotNan<f32>,
}

impl PipelineRasterizationStateCreateInfo {
    pub const DEFAULT: PipelineRasterizationStateCreateInfo =
        PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullModeFlags::NONE,
            depth_bias: DepthBias::Disabled,
            front_face: FrontFace::Clockwise,
            line_width: unsafe { mem::transmute(1.0f32) },
        };
}

impl Default for PipelineRasterizationStateCreateInfo {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Viewport {
    pub x: NotNan<f32>,
    pub y: NotNan<f32>,
    pub width: NotNan<f32>,
    pub height: NotNan<f32>,
    pub min_depth: NotNan<f32>,
    pub max_depth: NotNan<f32>,
}

impl From<(u32, u32)> for Viewport {
    fn from((w, h): (u32, u32)) -> Self {
        Viewport {
            x: 0.0.into(),
            y: 0.0.into(),
            width: (w as f32).into(),
            height: (h as f32).into(),
            min_depth: 0.0.into(),
            max_depth: 1.0.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ScissorRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PipelineViewports<'a> {
    Static { viewports: &'a [Viewport] },
    Dynamic,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PipelineScissors<'a> {
    Static { scissor_rects: &'a [ScissorRect] },
    Dynamic,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PipelineViewportStateCreateInfo<'a> {
    pub viewports: PipelineViewports<'a>,
    pub scissors: PipelineScissors<'a>,
}

impl<'a> Default for PipelineViewportStateCreateInfo<'a> {
    fn default() -> Self {
        PipelineViewportStateCreateInfo {
            scissors: PipelineScissors::Dynamic,
            viewports: PipelineViewports::Dynamic,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PipelineInputAssemblyStateCreateInfo {
    pub topology: PrimitiveTopology,
    pub primitive_restart_enable: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SampleShading {
    Disabled,
    Enabled { min_sample_shading: NotNan<f32> },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PipelineMultisampleStateCreateInfo {
    pub rasterization_samples: u32,
    pub sample_shading: SampleShading,
    //pub sample_mask: ???
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
}

impl Default for PipelineMultisampleStateCreateInfo {
    fn default() -> Self {
        PipelineMultisampleStateCreateInfo {
            rasterization_samples: 1,
            sample_shading: SampleShading::Disabled,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VertexInputBindingDescription {
    pub binding: u32,
    pub stride: u32,
    pub input_rate: VertexInputRate,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PipelineVertexInputStateCreateInfo<'a> {
    pub bindings: &'a [VertexInputBindingDescription],
    pub attributes: &'a [VertexInputAttributeDescription],
}

#[derive(Debug)]
pub struct PipelineLayoutCreateInfo<'a, 'rcx, R: RendererBackend> {
    pub descriptor_set_layouts: &'a [&'rcx R::DescriptorSetLayout],
    //pub push_constants: &'a [PushConstant]
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AttachmentDescription {
    pub format: Format,
    pub samples: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AttachmentLayoutCreateInfo<'a> {
    pub input_attachments: &'a [AttachmentDescription],
    pub depth_attachment: Option<AttachmentDescription>,
    pub color_attachments: &'a [AttachmentDescription],
    //pub resolve_attachments: &'a [AttachmentDescription]
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CompareOp {
    Never = 0,
    Less = 1,
    Equal = 2,
    LessOrEqual = 3,
    Greater = 4,
    NotEqual = 5,
    GreaterOrEqual = 6,
    Always = 7,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum StencilOp {
    Keep = 0,
    Zero = 1,
    Replace = 2,
    IncrementAndClamp = 3,
    DecrementAndClamp = 4,
    Invert = 5,
    IncrementAndWrap = 6,
    DecrementAndWrap = 7,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct StencilOpState {
    pub fail_op: StencilOp,
    pub pass_op: StencilOp,
    pub depth_fail_op: StencilOp,
    pub compare_op: CompareOp,
    pub compare_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum DepthBoundTest {
    Disabled,
    Enabled {
        min_depth_bounds: NotNan<f32>,
        max_depth_bounds: NotNan<f32>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum StencilTest {
    Disabled,
    Enabled {
        front: StencilOpState,
        back: StencilOpState,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PipelineDepthStencilStateCreateInfo {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: CompareOp,
    pub depth_bounds_test: DepthBoundTest,
    pub stencil_test: StencilTest,
}

impl Default for PipelineDepthStencilStateCreateInfo {
    fn default() -> Self {
        PipelineDepthStencilStateCreateInfo {
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: CompareOp::Less,
            depth_bounds_test: DepthBoundTest::Disabled,
            stencil_test: StencilTest::Disabled,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum LogicOp {
    Clear = 0,
    And = 1,
    AndReverse = 2,
    Copy = 3,
    AndInverted = 4,
    NoOp = 5,
    Xor = 6,
    Or = 7,
    Nor = 8,
    Equivalent = 9,
    Invert = 10,
    OrReverse = 11,
    CopyInverted = 12,
    OrInverted = 13,
    Nand = 14,
    Set = 15,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum BlendFactor {
    Zero = 0,
    One = 1,
    SrcColor = 2,
    OneMinusSrcColor = 3,
    DstColor = 4,
    OneMinusDstColor = 5,
    SrcAlpha = 6,
    OneMinusSrcAlpha = 7,
    DstAlpha = 8,
    OneMinusDstAlpha = 9,
    ConstantColor = 10,
    OneMinusConstantColor = 11,
    ConstantAlpha = 12,
    OneMinusConstantAlpha = 13,
    SrcAlphaSaturate = 14,
    Src1Color = 15,
    OneMinusSrc1Color = 16,
    Src1Alpha = 17,
    OneMinusSrc1Alpha = 18,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum BlendOp {
    Add = 0,
    Subtract = 1,
    ReverseSubtract = 2,
    Min = 3,
    Max = 4,
}

bitflags! {
    pub struct ColorComponentFlags: u32 {
        const R = 0x0000_0001;
        const G = 0x0000_0002;
        const B = 0x0000_0004;
        const A = 0x0000_0008;
        const RGBA = Self::R.bits | Self::G.bits | Self::B.bits  | Self::A.bits;
        const ALL = Self::R.bits | Self::G.bits | Self::B.bits  | Self::A.bits;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PipelineColorBlendAttachmentState {
    Disabled,
    Enabled {
        src_color_blend_factor: BlendFactor,
        dst_color_blend_factor: BlendFactor,
        color_blend_op: BlendOp,
        src_alpha_blend_factor: BlendFactor,
        dst_alpha_blend_factor: BlendFactor,
        alpha_blend_op: BlendOp,
        color_write_mask: ColorComponentFlags,
    },
}

impl PipelineColorBlendAttachmentState {
    pub const DISABLED: PipelineColorBlendAttachmentState =
        PipelineColorBlendAttachmentState::Disabled;
    pub const ALPHA_BLENDING: PipelineColorBlendAttachmentState =
        PipelineColorBlendAttachmentState::Enabled {
            color_blend_op: BlendOp::Add,
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::SrcAlpha,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_write_mask: ColorComponentFlags::ALL,
        };
}

impl Default for PipelineColorBlendAttachmentState {
    fn default() -> Self {
        PipelineColorBlendAttachmentState::Disabled
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PipelineColorBlendAttachments<'a> {
    All(&'a PipelineColorBlendAttachmentState),
    Separate(&'a [PipelineColorBlendAttachmentState]),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PipelineColorBlendStateCreateInfo<'a> {
    pub logic_op: Option<LogicOp>,
    pub attachments: PipelineColorBlendAttachments<'a>,
    pub blend_constants: [NotNan<f32>; 4],
}

#[derive(Debug)]
pub struct GraphicsPipelineCreateInfo<'a, 'rcx, R: RendererBackend> {
    pub shader_stages: &'a GraphicsPipelineShaderStages<'rcx, R>,
    pub vertex_input_state: &'a PipelineVertexInputStateCreateInfo<'a>,
    pub viewport_state: &'a PipelineViewportStateCreateInfo<'a>,
    pub rasterization_state: &'a PipelineRasterizationStateCreateInfo,
    pub multisample_state: &'a PipelineMultisampleStateCreateInfo,
    pub depth_stencil_state: &'a PipelineDepthStencilStateCreateInfo,
    pub input_assembly_state: &'a PipelineInputAssemblyStateCreateInfo,
    pub color_blend_state: &'a PipelineColorBlendStateCreateInfo<'a>,
    pub dynamic_state: DynamicStateFlags,
    pub pipeline_layout: &'a PipelineLayoutCreateInfo<'a, 'rcx, R>,
    pub attachment_layout: &'a AttachmentLayoutCreateInfo<'a>,
    pub additional: &'a R::GraphicsPipelineCreateInfoAdditional,
}

// rust issue #26925
impl<'a, 'rcx, R: RendererBackend> Clone for GraphicsPipelineCreateInfo<'a, 'rcx, R> {
    fn clone(&self) -> Self {
        // safe because TODO
        // has no dynamically allocated components or mutable refs
        unsafe { mem::transmute_copy(self) }
    }
}

pub struct BufferSlice<'a, R: RendererBackend> {
    pub buffer: &'a R::Buffer,
    pub offset: usize,
    pub size: usize,
}

/*
pub struct SubmitFrame<R: RendererBackend> {
    pub commands: Vec<Command<R>>,
}*/

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AliasScope {
    pub value: u64,
    pub mask: u64,
}

impl AliasScope {
    pub fn no_alias() -> AliasScope {
        AliasScope { value: 0, mask: 0 }
    }

    pub fn overlaps(&self, other: &AliasScope) -> bool {
        let m = self.mask & other.mask;
        (self.value & m) == (other.value & m)
    }
}

//--------------------------------------------------------------------------------------------------
pub trait Swapchain: Debug {
    fn size(&self) -> (u32, u32);
}
pub trait Buffer: Debug {}
pub trait Image: Debug {}
pub trait Framebuffer: Debug {}
pub trait DescriptorSetLayout: Debug {}
pub trait ShaderModule: Debug {}
pub trait GraphicsPipeline: Debug {}

pub struct DescriptorSet<'a, R: RendererBackend> {
    inner: &'a R::DescriptorSet,
}

/// V2 API
/// Some associated backend types (such as Framebuffers, or DescriptorSets) conceptually "borrow"
/// the referenced resources, and as such should have an associated lifetime parameter.
/// However, this cannot be expressed right now because of the lack of generic associated types
/// (a.k.a. associated type constructors, or ATCs).
pub trait RendererBackend: Sync {
    type Swapchain: Swapchain;
    type Framebuffer: Framebuffer;
    type Buffer: Buffer;
    type Image: Image;
    type DescriptorSet;
    type DescriptorSetLayout: DescriptorSetLayout;
    type ShaderModule: ShaderModule;
    type GraphicsPipeline: GraphicsPipeline;
    type GraphicsPipelineCreateInfoAdditional;

    /// Contains resources.
    type Arena: Sync;

    fn create_arena(&self) -> Self::Arena;

    /// Drops a group of resources in the arena.
    fn drop_arena(&self, arena: Self::Arena)
    where
        Self: Sized;

    fn create_swapchain<'a>(&self, arena: &'a Self::Arena) -> &'a Self::Swapchain
    where
        Self: Sized;
    fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx Self::Swapchain>;

    /// Creates an immutable image that cannot be modified by any operation (render, transfer, swaps or otherwise).
    /// Useful for long-lived texture data.
    fn create_immutable_image<'a>(
        &self,
        arena: &'a Self::Arena,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: &[u8],
    ) -> &'a Self::Image
    where
        Self: Sized;

    /// Creates an image containing uninitialized data.
    fn create_image<'a>(
        &self,
        arena: &'a Self::Arena,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a Self::Image
    where
        Self: Sized;

    /// Creates a framebuffer.
    fn create_framebuffer<'a>(
        &self,
        arena: &'a Self::Arena,
        color_attachments: &[&'a Self::Image],
        depth_stencil_attachment: Option<&'a Self::Image>,
    ) -> &'a Self::Framebuffer
    where
        Self: Sized;

    /// Creates an immutable buffer.
    fn create_immutable_buffer<'a>(
        &self,
        arena: &'a Self::Arena,
        size: u64,
        data: &[u8],
    ) -> &'a Self::Buffer
    where
        Self: Sized;

    /// Creates a buffer containing uninitialized data.
    fn create_buffer<'a>(&self, arena: &'a Self::Arena, size: u64) -> &'a Self::Buffer
    where
        Self: Sized;

    fn create_shader_module<'a>(
        &self,
        arena: &'a Self::Arena,
        data: &[u8],
        stage: ShaderStageFlags,
    ) -> &'a Self::ShaderModule
    where
        Self: Sized;

    fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a Self::Arena,
        create_info: &GraphicsPipelineCreateInfo<'_, 'a, Self>,
    ) -> &'a Self::GraphicsPipeline
    where
        Self: Sized;

    fn create_descriptor_set_layout<'a>(
        &self,
        arena: &'a Self::Arena,
        bindings: &[DescriptorSetLayoutBinding<'_>],
    ) -> &'a Self::DescriptorSetLayout
    where
        Self: Sized;

    /// Creates a new descriptor set. See trait documentation for explanation of unsafety.
    fn create_descriptor_set<'a>(
        &self,
        arena: &'a Self::Arena,
        layout: &Self::DescriptorSetLayout,
        descriptors: &[Descriptor<'a, Self>],
    ) -> &'a Self::DescriptorSet
    where
        Self: Sized;

    fn submit_frame<'a>(&self, commands: &[Command<'a, Self>])
    where
        Self: Sized;
}

//--------------------------------------------------------------------------------------------------
pub struct Arena<'rcx, R: RendererBackend> {
    backend: &'rcx R,
    inner_arena: Option<R::Arena>,
}

impl<'rcx, R: RendererBackend> Drop for Arena<'rcx, R> {
    fn drop(&mut self) {
        self.backend.drop_arena(self.inner_arena.take().unwrap())
    }
}

impl<'rcx, R: RendererBackend> Arena<'rcx, R> {
    pub fn inner_arena(&self) -> &R::Arena {
        self.inner_arena.as_ref().unwrap()
    }

    /// Creates a swapchain.
    #[inline]
    pub fn create_swapchain(&self) -> &R::Swapchain {
        self.backend.create_swapchain(self.inner_arena())
    }

    /// Creates a framebuffer.
    #[inline]
    pub fn create_framebuffer<'a>(
        &'a self,
        color_attachments: &[&'a R::Image],
        depth_stencil_attachment: Option<&'a R::Image>,
    ) -> &'a R::Framebuffer {
        self.backend.create_framebuffer(
            self.inner_arena(),
            color_attachments,
            depth_stencil_attachment,
        )
    }

    /// Creates a shader module.
    #[inline]
    pub fn create_shader_module(&self, data: &[u8], stage: ShaderStageFlags) -> &R::ShaderModule {
        self.backend
            .create_shader_module(self.inner_arena(), data, stage)
    }

    /// Creates a graphics pipeline.
    /// Pipeline = all shaders + input layout + output layout (expected buffers)
    /// Creation process?
    #[inline]
    pub fn create_graphics_pipeline<'a>(
        &'a self,
        create_info: &GraphicsPipelineCreateInfo<'_, 'a, R>,
    ) -> &'a R::GraphicsPipeline {
        self.backend
            .create_graphics_pipeline(self.inner_arena(), create_info)
    }

    /// Creates an image.
    /// Initial data is uploaded to the image memory, and will be visible to all operations
    /// from the current frame and after.
    /// (the first operation that depends on the image will block on transfer complete)
    #[inline]
    pub fn create_immutable_image(
        &self,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: &[u8],
    ) -> &R::Image {
        self.backend.create_immutable_image(
            self.inner_arena(),
            format,
            dimensions,
            mipcount,
            samples,
            usage,
            initial_data,
        )
    }

    /// Creates a scoped image.
    /// TODO document this stuff.
    #[inline]
    pub fn create_image(
        &self,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &R::Image {
        self.backend.create_image(
            self.inner_arena(),
            scope,
            format,
            dimensions,
            mipcount,
            samples,
            usage,
        )
    }

    /// Creates a GPU (device local) buffer.
    #[inline]
    pub fn create_buffer(&self, size: u64) -> &R::Buffer {
        self.backend.create_buffer(self.inner_arena(), size)
    }

    /// Creates a GPU (device local) buffer.
    #[inline]
    pub fn create_immutable_buffer(&self, size: u64, data: &[u8]) -> &R::Buffer {
        self.backend
            .create_immutable_buffer(self.inner_arena(), size, data)
    }

    #[inline]
    pub fn create_descriptor_set_layout<'a>(
        &'a self,
        bindings: &[DescriptorSetLayoutBinding],
    ) -> &'a R::DescriptorSetLayout {
        self.backend
            .create_descriptor_set_layout(self.inner_arena(), bindings)
    }

    pub fn create_descriptor_set<'a>(
        &'a self,
        layout: &R::DescriptorSetLayout,
        interface: impl DescriptorSetInterface<'a, R>,
    ) -> &'a R::DescriptorSet {
        struct Visitor<'a, R: RendererBackend> {
            descriptors: Vec<Descriptor<'a, R>>,
        }

        impl<'a, R: RendererBackend> DescriptorSetInterfaceVisitor<'a, R> for Visitor<'a, R> {
            fn visit_buffer(&mut self, binding: u32, buffer: &'a R::Buffer) {
                warn!("buffer {} {:?}", binding, buffer);
                self.descriptors.push(Descriptor::Buffer {
                    buffer,
                    offset: 0,
                    size: 0,
                })
            }
        }

        let mut visitor = Visitor {
            descriptors: Vec::new(),
        };

        interface.do_visit(&mut visitor);

        self.backend
            .create_descriptor_set(self.inner_arena(), layout, &visitor.descriptors)
    }
}

//--------------------------------------------------------------------------------------------------
pub struct Renderer<R: RendererBackend> {
    backend: R,
}

impl<R: RendererBackend> Renderer<R> {
    pub fn new(backend: R) -> Renderer<R> {
        Renderer { backend }
    }

    pub fn create_arena<'r>(&'r self) -> Arena<'r, R> {
        Arena {
            backend: &self.backend,
            inner_arena: Some(self.backend.create_arena()),
        }
    }

    /// Returns the default swapchain handle, if any.
    pub fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx R::Swapchain> {
        self.backend.default_swapchain()
    }

    /// Creates a command buffer.
    pub fn create_command_buffer<'cmd>(&self) -> CommandBuffer<'cmd, R> {
        CommandBuffer::new()
    }

    /// Signals the end of the current frame, and starts another.
    pub fn submit_frame<'cmd>(&self, command_buffers: Vec<CommandBuffer<'cmd, R>>) {
        let commands = sort_command_buffers(command_buffers);
        self.backend.submit_frame(&commands)
    }
}
