use crate::descriptor::DescriptorBinding;
use crate::format::Format;
use crate::framebuffer::FragmentOutputDescription;
use crate::vertex::IndexFormat;
use crate::vertex::VertexLayout;
use crate::Arena;
use crate::Backend;
use crate::Renderer;
pub use autograph_render_macros::PipelineInterface;
use bitflags::bitflags;
use ordered_float::NotNan;
use std::marker::PhantomData;
use std::mem;
use std::fmt::Debug;

//pub mod validate;

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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ShaderFormat {
    SpirV,
    BackendSpecific,
}

#[derive(Copy, Clone, Debug)]
pub struct GraphicsShaderStages<'a, 'spv, B: Backend> {
    //pub format: ShaderFormat,
    pub vertex: ShaderModule<'a, 'spv, B>,
    pub geometry: Option<ShaderModule<'a, 'spv, B>>,
    pub fragment: Option<ShaderModule<'a, 'spv, B>>,
    pub tess_eval: Option<ShaderModule<'a, 'spv, B>>,
    pub tess_control: Option<ShaderModule<'a, 'spv, B>>,
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
pub struct RasterisationState {
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: PolygonMode,
    pub cull_mode: CullModeFlags,
    pub depth_bias: DepthBias,
    pub front_face: FrontFace,
    pub line_width: NotNan<f32>,
}

impl RasterisationState {
    pub const DEFAULT: RasterisationState = RasterisationState {
        depth_clamp_enable: false,
        rasterizer_discard_enable: false,
        polygon_mode: PolygonMode::Fill,
        cull_mode: CullModeFlags::NONE,
        depth_bias: DepthBias::Disabled,
        front_face: FrontFace::Clockwise,
        line_width: unsafe { mem::transmute(1.0f32) },
    };
}

impl Default for RasterisationState {
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
pub enum Viewports<'a> {
    Static { viewports: &'a [Viewport] },
    Dynamic,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Scissors<'a> {
    Static { scissor_rects: &'a [ScissorRect] },
    Dynamic,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ViewportState<'a> {
    pub viewports: Viewports<'a>,
    pub scissors: Scissors<'a>,
}

impl<'a> Default for ViewportState<'a> {
    fn default() -> Self {
        ViewportState {
            scissors: Scissors::Dynamic,
            viewports: Viewports::Dynamic,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct InputAssemblyState {
    pub topology: PrimitiveTopology,
    pub primitive_restart_enable: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SampleShading {
    Disabled,
    Enabled { min_sample_shading: NotNan<f32> },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct MultisampleState {
    pub rasterization_samples: u32,
    pub sample_shading: SampleShading,
    //pub sample_mask: ???
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        MultisampleState {
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
pub struct VertexInputState<'a> {
    pub bindings: &'a [VertexInputBindingDescription],
    pub attributes: &'a [VertexInputAttributeDescription],
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AttachmentDescription {
    pub format: Format,
    pub samples: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AttachmentLayout<'a> {
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
pub struct DepthStencilState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: CompareOp,
    pub depth_bounds_test: DepthBoundTest,
    pub stencil_test: StencilTest,
}

impl Default for DepthStencilState {
    fn default() -> Self {
        DepthStencilState {
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
pub enum ColorBlendAttachmentState {
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

impl ColorBlendAttachmentState {
    pub const DISABLED: ColorBlendAttachmentState = ColorBlendAttachmentState::Disabled;
    pub const ALPHA_BLENDING: ColorBlendAttachmentState = ColorBlendAttachmentState::Enabled {
        color_blend_op: BlendOp::Add,
        src_color_blend_factor: BlendFactor::SrcAlpha,
        dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::SrcAlpha,
        dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
        color_write_mask: ColorComponentFlags::ALL,
    };
}

impl Default for ColorBlendAttachmentState {
    fn default() -> Self {
        ColorBlendAttachmentState::Disabled
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ColorBlendAttachments<'a> {
    All(&'a ColorBlendAttachmentState),
    Separate(&'a [ColorBlendAttachmentState]),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ColorBlendState<'a> {
    pub logic_op: Option<LogicOp>,
    pub attachments: ColorBlendAttachments<'a>,
    pub blend_constants: [NotNan<f32>; 4],
}

#[derive(Copy, Clone)]
pub struct GraphicsPipelineCreateInfo<'a, 'b, B: Backend> {
    /// Shaders
    pub shader_stages: &'b GraphicsShaderStages<'a, 'b, B>,
    //pub vertex_input_state: &'b VertexInputState<'b>,
    pub viewport_state: &'b ViewportState<'b>,
    pub rasterization_state: &'b RasterisationState,
    pub multisample_state: &'b MultisampleState,
    pub depth_stencil_state: &'b DepthStencilState,
    pub input_assembly_state: &'b InputAssemblyState,
    pub color_blend_state: &'b ColorBlendState<'b>,
    pub dynamic_state: DynamicStateFlags,
}

/*
/// Variant of [GraphicsPipelineCreateInfoTypeless] where some information is derived from types
/// passed to [Arena::create_graphics_pipeline].
#[derive(Copy, Clone)]
pub struct GraphicsPipelineCreateInfo<'a, 'b, B: Backend> {
    pub shader_stages: &'b GraphicsShaderStages<'a, 'b, B>,
    pub viewport_state: &'b ViewportState<'b>,
    pub rasterization_state: &'b RasterisationState,
    pub multisample_state: &'b MultisampleState,
    pub depth_stencil_state: &'b DepthStencilState,
    pub input_assembly_state: &'b InputAssemblyState,
    pub color_blend_state: &'b ColorBlendState<'b>,
}
*/

//--------------------------------------------------------------------------------------------------

/// Shader module.
///
/// We keep a reference to the SPIR-V bytecode for interface checking when building pipelines.
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct ShaderModule<'a, 'spv, B: Backend>(
    pub(crate) &'a B::ShaderModule,
    pub(crate) &'spv [u8],
);

impl<'a, 'spv, B: Backend> ShaderModule<'a, 'spv, B> {
    pub fn inner(&self) -> &'a B::ShaderModule {
        self.0
    }

    pub fn spirv_binary(&self) -> &'spv [u8] {
        self.1
    }
}

/// Graphics pipeline.
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct GraphicsPipelineTypeless<'a, B: Backend>(pub(crate) &'a B::GraphicsPipeline);

/*
impl<'a, B: Backend, T: PipelineInterface<'a, B>> GraphicsPipeline<'a,B,T> {
    pub fn root_signature(&self) -> PipelineSignatureTypeless<'a, B> {
        PipelineSignatureTypeless(self.0.signature)
    }
}*/

/*
// Type erasure for GraphicsPipelines
impl<'a, B: Backend, T: PipelineInterface<'a, B>> From<GraphicsPipeline<'a, B, T>>
    for GraphicsPipelineTypeless<'a, B>
{
    fn from(pipeline: GraphicsPipeline<'a, B, T>) -> Self {
        pipeline.0
    }
}*/

/// Complete description of a pipeline signature, including the description of sub-signatures.
/// Contrary to PipelineSignatureCreateInfo, this doesn't hold any backend signature object.
#[derive(Copy, Clone, Debug)]
pub struct SignatureDescription<'a> {
    pub inherited: &'a [&'a SignatureDescription<'a>],
    pub descriptors: &'a [DescriptorBinding<'a>],
    pub vertex_layouts: &'a [VertexLayout<'a>],
    pub fragment_outputs: &'a [FragmentOutputDescription],
    pub depth_stencil_fragment_output: Option<FragmentOutputDescription>,
    pub index_format: Option<IndexFormat>,
    pub is_root_fragment_output_signature: bool,
    pub is_root_vertex_input_signature: bool,
}

pub trait Signature<'a, B: Backend>: Copy + Clone + Debug {
    fn inner(&self) -> &'a B::Signature;
    fn description(&self) -> &SignatureDescription;
}

#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct TypedSignature<'a, B: Backend, T: PipelineInterface<'a, B>>(
    pub(crate) &'a B::Signature,
    pub(crate) PhantomData<&'a T>,
);

impl<'a, B: Backend, T: PipelineInterface<'a, B>> Signature<'a, B> for TypedSignature<'a, B, T> {
    fn inner(&self) -> &'a B::Signature {
        self.0
    }
    fn description(&self) -> &SignatureDescription {
        T::SIGNATURE
    }
}

#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct Arguments<'a, B: Backend, S: Signature<'a, B>> {
    pub(crate) arguments: &'a B::Arguments,
    pub(crate) signature: S,
}

#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct BareArguments<'a, B: Backend>(pub &'a B::Arguments);

impl<'a, B: Backend, S: Signature<'a, B>> Into<BareArguments<'a, B>> for Arguments<'a, B, S>
{
    fn into(self) -> BareArguments<'a, B> {
        BareArguments(self.arguments)
    }
}

/*impl<'a, B: Backend, S: Signature<'a, B>> Arguments<'a, B, S>  {
    fn inner(&self) -> &'a B::Arguments {
        self.arguments
    }
    fn signature(&self) -> S {
        self.signature
    }
}*/

pub type TypedArguments<'a, B, T> = Arguments<'a, B, TypedSignature<'a, B, T>>;

/// Graphics pipeline.
/// OR GraphicsPipeline<'a, B: Backend, S: Signature> ?

#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct GraphicsPipeline<'a, B: Backend, S: Signature<'a, B>> {
    pub(crate) inner: &'a B::GraphicsPipeline,
    pub(crate) signature: S,
}

pub type TypedGraphicsPipeline<'a, B, T> = GraphicsPipeline<'a, B, TypedSignature<'a, B, T>>;

///
/// Describes pipeline states to set before issuing a draw or compute call.
///
/// #### Custom derive
/// It is possible to automatically derive [PipelineInterface] for structs with named fields.
///
/// By default, the implementation is generic over the backend type, and expects the struct to have
/// one lifetime parameter (`'a`) and a single type parameter for the renderer backend
/// (`T: RendererBackend`).
///
/// Attributes define the interpretation of struct fields in terms of pipeline bindings.
/// The field must be convertible via [Into] to the target binding type:
/// * `descriptor_set(index=n)` specifies a descriptor set.
/// The field must implement `Into<DescriptorSet>`.
/// * `vertex_buffer` specifies a vertex buffer. The field must implement `Into<BufferTypeless>`.
/// * `index_buffer` specifies an index buffer. The field must implement `Into<BufferTypeless>`.
///   Annotating more than one field with this attribute is an error.
/// * `viewport` specifies a viewport. The field must implement `Into<Viewport>`.
/// * `scissor` specifies a scissor rectangle. The field must implement `Into<ScissorRect>`.
///
/// It is also possible to specify arrays of pipeline bindings:
/// * `descriptor_set_array(base_index=n)` specifies an array of descriptor sets.
///   The field must implement `IntoIterator<DescriptorSet>`.
/// * `vertex_buffer_array(base_index=n)` specifies an array of vertex buffers.
///   The field must implement `IntoIterator<BufferTypeless>`.
/// * `viewport_array(base_index=n)` specifies an array of viewports.
///   The field must implement `IntoIterator<Viewport>`.
/// * `scissor_array(base_index=n)` specifies an array of scissor rectangles.
///   The field must implement `IntoIterator<ScissorRect>`.
///
/// When a binding index is required, it is derived from the order of appearance of the binding in
/// the struct. For instance:
///
/// ```
/// #[derive(PipelineInterface)]
/// pub struct ExampleInterface<'a> {
///     #[descriptor_set] ds_a: DescriptorSet<'a>,     // index 0
///     ...
///     #[descriptor_set] ds_b: DescriptorSet<'a>,     // index 1
///     ...
///     #[descriptor_set_array] dss: DescriptorSet<'a>,   // indices 2..2+dss.len
///     ...
///     #[descriptor_set] ds_c: DescriptorSet<'a>,   // index dss.len
/// }
/// ```
///
/// #### Example
///
///```
/// #[derive(PipelineInterface)]
/// pub struct ExampleInterface<'a> {
///    #[framebuffer]
///    pub framebuffer: Framebuffer<'a>,
///    #[descriptor_set]
///    pub per_object: DescriptorSet<'a>,   // DS index 0
///    #[viewport]
///    pub viewport: Viewport,
///    // Buffer<T> implements Into<BufferTypeless>
///    #[vertex_buffer(base_location=0)]
///    pub vertex_buffer: Buffer<'a, [Vertex]>,  // VB index 0
///    #[descriptor_set]
///    pub other: DescriptorSet<'a>,  // DS index 1
/// }
/// ```
///
pub trait PipelineInterface<'a, B: Backend>: Sized {
    const SIGNATURE: &'static SignatureDescription<'static>;

    /// A 'static marker type that uniquely identifies Self: this is for getting a TypeId.
    type UniqueType: 'static;
    type IntoInterface: PipelineInterface<'a, B> + 'a;

    fn get_inherited_signatures(renderer: &'a Renderer<B>) -> Vec<&'a B::Signature>;
    fn into_arguments(
        self,
        signature: TypedSignature<'a, B, Self::IntoInterface>,
        arena: &'a Arena<B>,
    ) -> Arguments<'a, B, TypedSignature<'a, B, Self::IntoInterface>>;
}

impl<'a, B: Backend, P: PipelineInterface<'a, B>> PipelineInterface<'a, B>
    for Arguments<'a, B, TypedSignature<'a, B, P>>
{
    const SIGNATURE: &'static SignatureDescription<'static> = P::SIGNATURE;
    type UniqueType = P::UniqueType;
    type IntoInterface = P;

    fn get_inherited_signatures(renderer: &'a Renderer<B>) -> Vec<&'a B::Signature> {
        P::get_inherited_signatures(renderer)
    }

    fn into_arguments(
        self,
        _signature: TypedSignature<'a, B, P>,
        _arena: &'a Arena<B>,
    ) -> Arguments<'a, B, TypedSignature<'a, B, P>> {
        self.into()
    }
}
