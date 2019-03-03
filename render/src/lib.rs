//! Renderer manifesto:
//! * Easy to use
//! * Flexible
//! * Not too verbose
//! * Dynamic
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

// necessary for const NotNaN
#![feature(const_transmute)]
// hopefully stable soon
#![feature(try_from)]
extern crate log;

// Reexport nalgebra_glm types if requested
#[cfg(feature = "glm")]
pub use nalgebra_glm as glm;

pub mod buffer;
pub mod command;
pub mod descriptor;
pub mod format;
pub mod framebuffer;
pub mod image;
pub mod pipeline;
pub mod swapchain;
mod sync;
pub mod traits;
pub mod typedesc;
mod util;
pub mod vertex;
pub mod error;

pub use crate::{buffer::*, command::*, descriptor::*, format::*, image::*, util::*};

// re-export macros
pub use autograph_shader_macros::{
    glsl_compute, glsl_fragment, glsl_geometry, glsl_tess_control, glsl_tess_eval, glsl_vertex,
    include_shader,
};

use crate::{
    pipeline::{
        Arguments, GraphicsPipeline, GraphicsPipelineCreateInfo, Scissor, ShaderModule,
        ShaderStageFlags, Viewport,
    },
    swapchain::Swapchain,
    vertex::{IndexBufferDescriptor, VertexBufferDescriptor},
};
use std::{any::TypeId, collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData, mem};
//use crate::pipeline::validate::ValidationError;
use crate::pipeline::{
    validate::validate_spirv_graphics_pipeline, ArgumentBlock, BareArgumentBlock,
    GraphicsShaderStages, Signature, SignatureDescription, TypedSignature,
};
use std::sync::Mutex;

//--------------------------------------------------------------------------------------------------

/// Currently unused.
#[derive(Copy, Clone, Debug)]
pub enum MemoryType {
    DeviceLocal,
    HostUpload,
    HostReadback,
}

/// Currently unused.
#[derive(Copy, Clone, Debug)]
pub enum Queue {
    Graphics,
    Compute,
    Transfer,
}

//--------------------------------------------------------------------------------------------------

/// A contiguous range in the sorted command stream inside which a resource should not be aliased.
///
/// An AliasScope is defined using a mask and a value (similarly to IP subnet masks, for example):
/// a command with sortkey `s` is inside the range if `s & mask == value`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AliasScope {
    pub value: u64,
    pub mask: u64,
}

impl AliasScope {
    /// Returns an AliasScope encompassing the whole command stream.
    pub fn no_alias() -> AliasScope {
        AliasScope { value: 0, mask: 0 }
    }

    /// Returns true if this scope overlaps the other.
    pub fn overlaps(&self, other: &AliasScope) -> bool {
        let m = self.mask & other.mask;
        (self.value & m) == (other.value & m)
    }
}

//--------------------------------------------------------------------------------------------------

pub trait Instance<B: Backend> {
    /// Creates a new empty Arena.
    unsafe fn create_arena(&self) -> Box<B::Arena>;

    /// Drops an arena and all the objects it owns.
    unsafe fn drop_arena(&self, arena: Box<B::Arena>);

    /// See [Renderer::create_swapchain](crate::Renderer::create_swapchain).
    unsafe fn create_swapchain<'a>(&self, arena: &'a B::Arena) -> &'a B::Swapchain;

    /// See [Renderer::default_swapchain](crate::Renderer::default_swapchain).
    unsafe fn default_swapchain<'a>(&'a self) -> Option<&'a B::Swapchain>;

    /// Creates an immutable image that cannot be modified by any operation (render, transfer, swaps or otherwise).
    /// Useful for long-lived texture data.
    unsafe fn create_immutable_image<'a>(
        &self,
        arena: &'a B::Arena,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: &[u8],
    ) -> &'a B::Image;

    /// Creates an image containing uninitialized data.
    unsafe fn create_image<'a>(
        &self,
        arena: &'a B::Arena,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a B::Image;

    /// Updates a region of an image.
    ///
    /// This function assumes that the format of data matches the internal format of the image.
    /// No conversion is performed.
    unsafe fn update_image(
        &self,
        image: &B::Image,
        min_extent: (u32, u32, u32),
        max_extent: (u32, u32, u32),
        data: &[u8],
    );

    /// TODO
    unsafe fn create_immutable_buffer<'a>(
        &self,
        arena: &'a B::Arena,
        size: u64,
        data: &[u8],
    ) -> &'a B::Buffer;

    /// TODO
    unsafe fn create_buffer<'a>(&self, arena: &'a B::Arena, size: u64) -> &'a B::Buffer;

    unsafe fn create_shader_module<'a>(
        &self,
        arena: &'a B::Arena,
        spirv: &'_ [u8],
        stage: ShaderStageFlags,
    ) -> &'a B::ShaderModule;

    unsafe fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a B::Arena,
        root_signature: &'a B::Signature,
        root_signature_description: &SignatureDescription,
        create_info: &GraphicsPipelineCreateInfo<'a, '_, B>,
    ) -> &'a B::GraphicsPipeline;

    unsafe fn create_signature<'a>(
        &'a self,
        arena: &'a B::Arena,
        inherited: &[&'a B::Signature],
        description: &SignatureDescription,
    ) -> &'a B::Signature;

    unsafe fn create_argument_block<'a, 'b>(
        &self,
        arena: &'a B::Arena,
        signature: &'a B::Signature,
        //description: &SignatureDescription,
        arguments: impl IntoIterator<Item = BareArgumentBlock<'a, B>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, B>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferDescriptor<'a, 'b, B>>,
        index_buffer: Option<IndexBufferDescriptor<'a, B>>,
        render_targets: impl IntoIterator<Item = RenderTargetView<'a, B>>,
        depth_stencil_render_target: Option<RenderTargetView<'a, B>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = Scissor>,
    ) -> &'a B::ArgumentBlock;

    unsafe fn create_host_reference<'a>(
        &self,
        arena: &'a B::Arena,
        data: &'a [u8],
    ) -> &'a B::HostReference;

    /// Sends commands to the GPU for execution, and ends the current frame.
    /// Uploads all referenced host data to the GPU and releases the borrows.
    ///
    /// Precondition: the command list should be sorted by sortkey.
    unsafe fn submit_frame<'a>(&self, commands: &[Command<'a, B>]);
}

/// Trait implemented by renderer backends.
///
/// The `RendererBackend` trait provides an interface to create graphics resources and send commands
/// to one (TODO or more) GPU.
/// It has a number of associated types for various kinds of graphics objects.
/// It serves as an abstraction layer over a graphics API.
///
/// See the [autograph_render_gl] crate for an example implementation.
pub trait Backend:
    Copy + Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Hash + 'static
{
    // Some associated backend types (such as Framebuffers, or DescriptorSets) conceptually "borrow"
    // the referenced resources, and as such should have an associated lifetime parameter.
    // However, this cannot be expressed right now because of the lack of generic associated types
    // (a.k.a. associated type constructors, or ATCs).
    type Instance: Instance<Self>;
    type Arena;
    type Swapchain: Sync + Debug + traits::Swapchain;
    type Image: Sync + Debug;
    type Buffer: Sync + Debug;
    type ShaderModule: Sync + Debug;
    type GraphicsPipeline: Sync + Debug;
    type Signature: Sync + Debug;
    type ArgumentBlock: Sync + Debug;
    type HostReference: Sync + Debug;
}

/// Dummy backend for testing purposes.
///
/// Should this be in render-test?
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DummyBackend;

#[derive(Debug)]
pub struct DummySwapchain;

impl traits::Swapchain for DummySwapchain {
    fn size(&self) -> (u32, u32) {
        unimplemented!()
    }
}

impl Backend for DummyBackend {
    type Instance = DummyInstance;
    type Arena = ();
    type Swapchain = DummySwapchain;
    type Image = ();
    type Buffer = ();
    type ShaderModule = ();
    type GraphicsPipeline = ();
    type Signature = ();
    type ArgumentBlock = ();
    type HostReference = ();
}

/// Dummy instance for testing purposes.
///
/// All functions panic when called.
pub struct DummyInstance;

impl Instance<DummyBackend> for DummyInstance {
    unsafe fn create_arena(&self) -> Box<()> {
        unimplemented!()
    }

    unsafe fn drop_arena(&self, _arena: Box<()>) {
        unimplemented!()
    }

    unsafe fn create_swapchain<'a>(&self, _arena: &'a ()) -> &'a DummySwapchain {
        unimplemented!()
    }

    unsafe fn default_swapchain<'a>(&'a self) -> Option<&DummySwapchain> {
        unimplemented!()
    }

    unsafe fn create_immutable_image<'a>(
        &self,
        _arena: &'a (),
        _format: Format,
        _dimensions: Dimensions,
        _mipcount: MipmapsCount,
        _samples: u32,
        _usage: ImageUsageFlags,
        _initial_data: &[u8],
    ) -> &'a () {
        unimplemented!()
    }

    unsafe fn create_image<'a>(
        &self,
        _arena: &'a (),
        _scope: AliasScope,
        _format: Format,
        _dimensions: Dimensions,
        _mipcount: MipmapsCount,
        _samples: u32,
        _usage: ImageUsageFlags,
    ) -> &'a () {
        unimplemented!()
    }

    unsafe fn update_image(
        &self,
        _image: &(),
        _min_extent: (u32, u32, u32),
        _max_extent: (u32, u32, u32),
        _data: &[u8],
    ) {
        unimplemented!()
    }

    unsafe fn create_immutable_buffer<'a>(
        &self,
        _arena: &'a (),
        _size: u64,
        _data: &[u8],
    ) -> &'a () {
        unimplemented!()
    }

    unsafe fn create_buffer<'a>(&self, _arena: &'a (), _size: u64) -> &'a () {
        unimplemented!()
    }

    unsafe fn create_shader_module<'a>(
        &self,
        _arena: &'a (),
        _spirv: &[u8],
        _stage: ShaderStageFlags,
    ) -> &'a () {
        unimplemented!()
    }

    unsafe fn create_graphics_pipeline<'a>(
        &self,
        _arena: &'a (),
        _root_signature: &'a (),
        _root_signature_description: &SignatureDescription,
        _create_info: &GraphicsPipelineCreateInfo<DummyBackend>,
    ) -> &'a () {
        unimplemented!()
    }

    unsafe fn create_signature<'a>(
        &'a self,
        _arena: &'a (),
        _inherited: &[&()],
        _description: &SignatureDescription,
    ) -> &'a () {
        unimplemented!()
    }

    unsafe fn create_argument_block<'a, 'b>(
        &self,
        _arena: &'a (),
        _signature: &'a (),
        _arguments: impl IntoIterator<Item = BareArgumentBlock<'a, DummyBackend>>,
        _descriptors: impl IntoIterator<Item = Descriptor<'a, DummyBackend>>,
        _vertex_buffers: impl IntoIterator<Item = VertexBufferDescriptor<'a, 'b, DummyBackend>>,
        _index_buffer: Option<IndexBufferDescriptor<'a, DummyBackend>>,
        _render_targets: impl IntoIterator<Item = RenderTargetView<'a, DummyBackend>>,
        _depth_stencil_render_target: Option<RenderTargetView<'a, DummyBackend>>,
        _viewports: impl IntoIterator<Item = Viewport>,
        _scissors: impl IntoIterator<Item = Scissor>,
    ) -> &'a () {
        unimplemented!()
    }

    unsafe fn create_host_reference<'a>(&self, _arena: &'a (), _data: &'a [u8]) -> &'a () {
        unimplemented!()
    }

    unsafe fn submit_frame<'a>(&self, _commands: &[Command<'a, DummyBackend>]) {
        unimplemented!()
    }
}

//--------------------------------------------------------------------------------------------------

/// An allocator and container for renderer resources.
///
/// Arenas are allocators specialized for renderer resources: most objects created by the
/// renderer backend are allocated and owned by arenas, and are released all at once
/// when the arena is dropped.
/// The lifetime of most objects created by the renderer are bound to an arena,
/// and those objects cannot be dropped individually.
///
/// Typically, an application has one arena per resource lifetime. For instance,
/// an application could have the following arenas, sorted from long-lived to short-lived:
/// * an arena for long-lived resources, such as immutable textures,
/// * an arena for hot-reloadable resources, destroyed and recreated on user input or filesystem events,
/// * an arena for swapchain-related resources, destroyed and recreated when the swapchain is resized,
/// * an arena for resources that live for the current frame only.
///
/// This type is a wrapper around [RendererBackend::Arena] that drops the arena
/// when it goes out of scope.
pub struct Arena<'r, B: Backend> {
    renderer: &'r Renderer<B>,
    instance: &'r B::Instance,
    inner: Option<Box<B::Arena>>,
}

impl<'r, B: Backend> Drop for Arena<'r, B> {
    fn drop(&mut self) {
        unsafe { self.instance.drop_arena(self.inner.take().unwrap()) }
    }
}

impl<'r, B: Backend> Arena<'r, B> {
    pub fn inner(&self) -> &B::Arena {
        self.inner.as_ref().unwrap()
    }

    /// Creates a swapchain.
    #[inline]
    pub fn create_swapchain(&self) -> Swapchain<B> {
        Swapchain(unsafe { self.instance.create_swapchain(&self.inner()) })
    }

    /// Creates a shader module from SPIR-V bytecode.
    #[inline]
    pub fn create_shader_module<'a, 'spv>(
        &'a self,
        data: &'spv [u8],
        stage: ShaderStageFlags,
    ) -> ShaderModule<'a, 'spv, B> {
        ShaderModule(
            unsafe {
                self.instance
                    .create_shader_module(&self.inner(), data, stage)
            },
            data,
        )
    }

    /// Equivalent to `create_shader_module(data, ShaderStageFlags::VERTEX)`
    #[inline]
    pub fn create_vertex_shader_module<'a, 'spv>(
        &'a self,
        data: &'spv [u8],
    ) -> ShaderModule<'a, 'spv, B> {
        self.create_shader_module(data, ShaderStageFlags::VERTEX)
    }

    /// Equivalent to `create_shader_module(data, ShaderStageFlags::FRAGMENT)`
    #[inline]
    pub fn create_fragment_shader_module<'a, 'spv>(
        &'a self,
        data: &'spv [u8],
    ) -> ShaderModule<'a, 'spv, B> {
        self.create_shader_module(data, ShaderStageFlags::FRAGMENT)
    }

    /// Equivalent to `create_shader_module(data, ShaderStageFlags::GEOMETRY)`
    #[inline]
    pub fn create_geometry_shader_module<'a, 'spv>(
        &'a self,
        data: &'spv [u8],
    ) -> ShaderModule<'a, 'spv, B> {
        self.create_shader_module(data, ShaderStageFlags::GEOMETRY)
    }

    /// Equivalent to `create_shader_module(data, ShaderStageFlags::TESS_CONTROL)`
    #[inline]
    pub fn create_tess_control_shader_module<'a, 'spv>(
        &'a self,
        data: &'spv [u8],
    ) -> ShaderModule<'a, 'spv, B> {
        self.create_shader_module(data, ShaderStageFlags::TESS_CONTROL)
    }

    /// Equivalent to `create_shader_module(data, ShaderStageFlags::TESS_EVAL)`
    #[inline]
    pub fn create_tess_eval_shader_module<'a, 'spv>(
        &'a self,
        data: &'spv [u8],
    ) -> ShaderModule<'a, 'spv, B> {
        self.create_shader_module(data, ShaderStageFlags::TESS_EVAL)
    }

    /// Shorthand to create a `GraphicsShaderStages` object with a vertex and a fragment shader.
    #[inline]
    pub fn create_vertex_fragment_shader_stages<'a, 'spv>(
        &'a self,
        vertex_shader: &'spv [u8],
        fragment_shader: &'spv [u8],
    ) -> GraphicsShaderStages<'a, 'spv, B> {
        let vert = self.create_vertex_shader_module(vertex_shader);
        let frag = self.create_fragment_shader_module(fragment_shader);
        GraphicsShaderStages {
            vertex: vert,
            geometry: None,
            fragment: frag.into(),
            tess_eval: None,
            tess_control: None,
        }
    }

    /// Creates a graphics pipeline given the pipeline description passed in create_info
    /// and information derived from the pipeline interface type.
    pub fn create_graphics_pipeline<'a, P: Arguments<'a, B>>(
        &'a self,
        create_info: &GraphicsPipelineCreateInfo<'a, '_, B>,
    ) -> GraphicsPipeline<'a, B, TypedSignature<'a, B, P>> {
        let root_signature = self.renderer.get_cached_signature::<P>();

        // validate the pipeline
        let validation_result =
            validate_spirv_graphics_pipeline(root_signature.description(), &create_info);
        if let Err(es) = validation_result {
            for e in es {
                log::error!("validation error: {}", e);
            }
            panic!("graphics pipeline validation failed");
        }

        GraphicsPipeline {
            inner: unsafe {
                self.instance.create_graphics_pipeline(
                    &self.inner(),
                    root_signature.0,
                    P::SIGNATURE,
                    &create_info,
                )
            },
            signature: root_signature,
        }
    }

    /// Creates an immutable image that cannot be modified by any operation
    /// (render, transfer, swaps or otherwise).
    /// Useful for long-lived texture data.
    /// Initial data is uploaded to the image memory, and will be visible to all operations
    /// from the current frame and after.
    /// The first operation that depends on the image will block until the initial data upload
    /// is complete.
    #[inline]
    pub fn create_immutable_image(
        &self,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: &[u8],
    ) -> Image<B> {
        Image(unsafe {
            self.instance.create_immutable_image(
                &self.inner(),
                format,
                dimensions,
                mipcount,
                samples,
                usage,
                initial_data,
            )
        })
    }

    /// Creates an image containing uninitialized data.
    ///
    /// If `scope` is not `AliasScope::no_alias()`, the image is considered _aliasable_, meaning
    /// that the memory backing this image can be shared between multiple image objects.
    /// The image does not retain its contents between frames,
    /// and should only be accessed within the specified scope.
    /// This is suitable for transient image data that is not used during the entirety of a frame.
    ///
    /// See also [AliasScope].
    #[inline]
    pub fn create_image<'a>(
        &'a self,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> Image<'a, B> {
        Image(unsafe {
            self.instance.create_image(
                &self.inner(),
                scope,
                format,
                dimensions,
                mipcount,
                samples,
                usage,
            )
        })
    }

    /// Creates an image suitable for use exclusively as a render target (color attachment image),
    /// and usable in the whole command stream (unaliasable).
    ///
    /// The created image has one mip level, and cannot be used as a sampled image.
    ///
    /// Equivalent to `create_image(AliasScope::no_alias(), format, dimensions.into(), MipmapsCount::One, samples, ImageUsageFlags::COLOR_ATTACHMENT)`
    #[inline]
    pub fn create_unaliasable_render_target(
        &self,
        format: Format,
        dimensions: (u32, u32),
        samples: u32,
    ) -> Image<B> {
        self.create_image(
            AliasScope::no_alias(),
            format,
            dimensions.into(),
            MipmapsCount::One,
            samples,
            ImageUsageFlags::COLOR_ATTACHMENT,
        )
    }

    /*/// Creates an image suitable (but not optimal) for every use.
    #[inline]
    pub fn create_default_image_2d(&self, format: Format, dimensions: (u32, u32), mip_levels: MipmapsCount, samples: u32) -> Image<B>
    {

    }*/

    /// Creates a GPU (device local) buffer.
    #[inline]
    pub fn create_buffer_typeless(&self, size: u64) -> BufferTypeless<B> {
        BufferTypeless(unsafe { self.instance.create_buffer(&self.inner(), size) })
    }

    /// Creates a GPU (device local) buffer.
    #[inline]
    pub fn create_immutable_buffer_typeless(&self, size: u64, data: &[u8]) -> BufferTypeless<B> {
        BufferTypeless(unsafe {
            self.instance
                .create_immutable_buffer(&self.inner(), size, data)
        })
    }

    /// Creates an immutable, device-local GPU buffer containing an object of type T.
    #[inline]
    pub fn upload<T: Copy + 'static>(&self, data: &T) -> Buffer<B, T> {
        let size = mem::size_of::<T>();
        let bytes = unsafe { ::std::slice::from_raw_parts(data as *const T as *const u8, size) };

        Buffer(
            unsafe {
                self.instance
                    .create_immutable_buffer(&self.inner(), size as u64, bytes)
            },
            PhantomData,
        )
    }

    /// Creates an immutable, device-local GPU buffer containing an array of objects of type T.
    #[inline]
    pub fn upload_slice<T: Copy + 'static>(&self, data: &[T]) -> Buffer<B, [T]> {
        let size = mem::size_of_val(data);
        let bytes = unsafe { ::std::slice::from_raw_parts(data.as_ptr() as *const u8, size) };

        Buffer(
            unsafe {
                self.instance
                    .create_immutable_buffer(&self.inner(), size as u64, bytes)
            },
            PhantomData,
        )
    }

    /// Creates an immutable, device-local GPU buffer containing an array of objects of type T.
    #[inline]
    pub fn host_reference<'a, T: Copy + 'static>(&'a self, data: &'a T) -> HostReference<'a, B, T> {
        let size = mem::size_of::<T>();
        let bytes = unsafe { ::std::slice::from_raw_parts(data as *const T as *const u8, size) };

        HostReference(
            unsafe { self.instance.create_host_reference(&self.inner(), bytes) },
            PhantomData,
        )
    }

    /// Creates an immutable, device-local GPU buffer containing an array of objects of type T.
    #[inline]
    pub fn host_slice<'a, T: Copy + 'static>(&'a self, data: &'a [T]) -> HostReference<'a, B, T> {
        let size = mem::size_of_val(data);
        let bytes = unsafe { ::std::slice::from_raw_parts(data.as_ptr() as *const u8, size) };

        HostReference(
            unsafe { self.instance.create_host_reference(&self.inner(), bytes) },
            PhantomData,
        )
    }

    /// Creates an _argument block_.
    ///
    ///
    pub fn create_argument_block<'a, 'b, S: Signature<'a, B>>(
        &'a self,
        signature: S,
        inherited: impl IntoIterator<Item = BareArgumentBlock<'a, B>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, B>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferDescriptor<'a, 'b, B>>,
        index_buffer: Option<IndexBufferDescriptor<'a, B>>,
        render_targets: impl IntoIterator<Item = RenderTargetView<'a, B>>,
        depth_stencil_render_target: Option<RenderTargetView<'a, B>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = Scissor>,
    ) -> ArgumentBlock<'a, B, S> {
        ArgumentBlock {
            arguments: unsafe {
                self.instance.create_argument_block(
                    &self.inner(),
                    signature.inner(),
                    //signature.description(),
                    inherited,
                    descriptors,
                    vertex_buffers,
                    index_buffer,
                    render_targets,
                    depth_stencil_render_target,
                    viewports,
                    scissors,
                )
            },
            signature,
        }
    }

    pub fn create_typed_argument_block<'a, 'b, T: Arguments<'a, B>>(
        &'a self,
        args: T,
    ) -> ArgumentBlock<'a, B, TypedSignature<'a, B, T::IntoInterface>> {
        let sig = self.renderer.get_cached_signature::<T::IntoInterface>();
        args.into_block(sig, self)
    }
}

//--------------------------------------------------------------------------------------------------

/// Renderer.
///
/// This is the main interface for interacting with a backend.
/// All GPU resources (images, buffers, etc.) are allocated in _arenas_, and all elements of
/// an arena are dropped at the same time. The [create_arena] method creates a new arena.
///
/// Commands (draw, compute, upload...) are not sent immediately to the GPU.
/// Instead, they are first collected in _command buffers_.
/// Command buffers are created using the [create_command_buffer] method.
/// Once you have finished filling command buffers, they can be submitted all at once to the GPU
/// via the [submit_frame] method.
///
/// Note that the final submission order of commands to the GPU is defined by their associated
/// _sort key_. See (TODO) for more info.
pub struct Renderer<B: Backend> {
    /// Backend instance
    instance: B::Instance,
    /// Arena for long-lived or cached objects, such as pipeline signatures
    default_arena: Option<Box<B::Arena>>,
    /// Cache of pipeline signatures
    signature_cache: Mutex<HashMap<TypeId, *const B::Signature>>,
}

impl<B: Backend> Renderer<B> {
    /// Creates a new renderer with the specified backend.
    pub fn new(instance: B::Instance) -> Renderer<B> {
        let default_arena = unsafe { instance.create_arena() };
        Renderer {
            instance,
            default_arena: Some(default_arena),
            signature_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn create_arena(&self) -> Arena<B> {
        Arena {
            renderer: self,
            instance: &self.instance,
            inner: Some(unsafe { self.instance.create_arena() }),
        }
    }

    /// Returns or creates the pipeline signature associated to the pipeline interface type.
    pub fn get_cached_signature<'r, P: Arguments<'r, B>>(&'r self) -> TypedSignature<'r, B, P> {
        let typeid = TypeId::of::<P::UniqueType>();
        let cached = self.signature_cache.lock().unwrap().get(&typeid).cloned();
        if let Some(cached) = cached {
            unsafe { TypedSignature(&*cached, PhantomData) }
        } else {
            // signature not created yet
            let inherited = P::get_inherited_signatures(self);
            let sig = unsafe {
                self.instance.create_signature(
                    self.default_arena.as_ref().unwrap(),
                    &inherited,
                    P::SIGNATURE,
                )
            };
            self.signature_cache
                .lock()
                .unwrap()
                .insert(typeid, sig as *const _);
            TypedSignature(sig, PhantomData)
        }
    }

    /// Returns the default swapchain if there is one.
    pub fn default_swapchain(&self) -> Option<Swapchain<B>> {
        unsafe { self.instance.default_swapchain().map(|s| Swapchain(s)) }
    }

    /// Creates a command buffer.
    pub fn create_command_buffer<'cmd>(&self) -> CommandBuffer<'cmd, B> {
        CommandBuffer::new()
    }

    /// Submits the given command buffers for rendering and ends the current frame.
    ///
    /// Frame-granularity synchronization points happen in this call.
    /// A new frame is implicitly started after this call.
    pub fn submit_frame<'a>(
        &self,
        command_buffers: impl IntoIterator<Item = CommandBuffer<'a, B>>,
    ) {
        let commands = sort_command_buffers(command_buffers);
        unsafe { self.instance.submit_frame(&commands) }
    }
}
