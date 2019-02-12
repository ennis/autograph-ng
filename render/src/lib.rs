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
#![feature(const_type_id)]
extern crate log;

// Reexport nalgebra_glm types if requested
#[cfg(feature = "glm-types")]
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

/*
pub use self::buffer::Buffer;
pub use self::buffer::BufferData;
pub use self::buffer::BufferTypeless;
pub use self::buffer::BufferSlice;
pub use self::buffer::StructuredBufferData;

pub use self::image::Dimensions;
pub use self::image::Image;
pub use self::image::SampledImage;
pub use self::image::SamplerDescription;
pub use self::image::SamplerMipmapMode;
pub use self::image::SamplerAddressMode;
pub use self::image::MipmapsCount;
pub use self::image::Filter;
pub use self::image::get_texture_mip_map_count;

pub use self::pipeline::PipelineInterface;
pub use self::pipeline::Viewport;
pub use self::pipeline::ScissorRect;
pub use self::pipeline::Scissors;
pub use self::pipeline::Viewports;
pub use self::pipeline::ViewportState;
pub use self::pipeline::GraphicsPipelineCreateInfo;
pub use self::pipeline::GraphicsPipeline;
pub use self::pipeline::ShaderModule;
pub use self::pipeline::VertexInputAttributeDescription;
pub use self::pipeline::VertexInputBindingDescription;
pub use self::pipeline::PrimitiveTopology;

pub use self::format::Format;
pub use self::format::FormatInfo;
pub use self::format::NumericFormat;
pub use self::format::ComponentLayout;
*/

use self::swapchain::Swapchain;

pub use self::buffer::*;
pub use self::command::*;
pub use self::descriptor::*;
pub use self::format::*;
pub use self::image::*;
pub use self::util::*;
// re-export macros
pub use autograph_shader_macros::{
    glsl_compute, glsl_fragment, glsl_geometry, glsl_tess_control, glsl_tess_eval, glsl_vertex,
    include_combined_shader, include_shader,
};
use std::marker::PhantomData;

//use crate::pipeline::build_vertex_input_interface;
//use crate::pipeline::validate::validate_graphics;
use crate::framebuffer::RenderTargetDescriptor;
use crate::pipeline::DynamicStateFlags;
use crate::pipeline::GraphicsPipeline;
use crate::pipeline::GraphicsPipelineCreateInfo;
use crate::pipeline::PipelineInterface;
use crate::pipeline::ScissorRect;
use crate::pipeline::ShaderModule;
use crate::pipeline::ShaderStageFlags;
use crate::pipeline::Viewport;
use crate::vertex::IndexBufferDescriptor;
use crate::vertex::VertexBufferDescriptor;
use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::mem;
//use crate::pipeline::validate::ValidationError;
use crate::pipeline::Arguments;
use crate::pipeline::Signature;
use crate::pipeline::SignatureDescription;
use crate::pipeline::TypedSignature;
use std::sync::Mutex;
use crate::pipeline::BareArguments;

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
    ///
    /// See [Arena::create_image](crate::arena::Arena::create_image).
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

    /*/// See [Arena::create_framebuffer](crate::arena::Arena::create_framebuffer).
    fn create_framebuffer<'a>(
        &self,
        arena: &'a Arena<B>,
        color_attachments: &[handle::Image<'a>],
        depth_stencil_attachment: Option<handle::Image<'a>>,
    ) -> handle::Framebuffer<'a>;*/

    /// TODO
    unsafe fn create_immutable_buffer<'a>(
        &self,
        arena: &'a B::Arena,
        size: u64,
        data: &[u8],
    ) -> &'a B::Buffer;

    /// TODO
    unsafe fn create_buffer<'a>(&self, arena: &'a B::Arena, size: u64) -> &'a B::Buffer;

    /// See [Arena::create_shader_module](crate::arena::Arena::create_shader_module).
    unsafe fn create_shader_module<'a>(
        &self,
        arena: &'a B::Arena,
        spirv: &'_ [u8],
        stage: ShaderStageFlags,
    ) -> &'a B::ShaderModule;

    /// See [Arena::create_graphics_pipeline](crate::arena::Arena::create_graphics_pipeline).
    unsafe fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a B::Arena,
        root_signature: &'a B::Signature,
        root_signature_description: &SignatureDescription,
        create_info: &GraphicsPipelineCreateInfo<'a, '_, B>,
    ) -> &'a B::GraphicsPipeline;

    ///
    unsafe fn create_signature<'a>(
        &'a self,
        arena: &'a B::Arena,
        inherited: &[&'a B::Signature],
        description: &SignatureDescription,
    ) -> &'a B::Signature;

    /// Creates a new argument group,
    /// which describes a set of resources to be bound to the graphics
    /// pipeline, and state to be set.
    unsafe fn create_arguments<'a, 'b>(
        &self,
        arena: &'a B::Arena,
        signature: &'a B::Signature,
        //description: &SignatureDescription,
        arguments: impl IntoIterator<Item = BareArguments<'a, B>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, B>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferDescriptor<'a, 'b, B>>,
        index_buffer: Option<IndexBufferDescriptor<'a, B>>,
        render_targets: impl IntoIterator<Item = RenderTargetDescriptor<'a, B>>,
        depth_stencil_render_target: Option<RenderTargetDescriptor<'a, B>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = ScissorRect>,
    ) -> &'a B::Arguments;

    /// Creates a reference to host data that is going to be used in pipeline arguments.
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
    type Arguments: Sync + Debug;
    type HostReference: Sync + Debug;
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

    /*/// Creates a graphics pipeline given the pipeline description passed in create_info.
    #[inline]
    pub fn create_graphics_pipeline_typeless<'a>(
        &'a self,
        create_info: &GraphicsPipelineCreateInfoTypeless<'a, '_, B>,
    ) -> GraphicsPipelineTypeless<'a, B> {
        GraphicsPipelineTypeless {
            pipeline: unsafe {
                self.instance.create_graphics_pipeline( & self.inner(), create_info)
            },
            signature: create_info.root_signature.0
        }
    }*/

    /// Creates a graphics pipeline given the pipeline description passed in create_info
    /// and information derived from the pipeline interface type.
    pub fn create_graphics_pipeline<'a, P: PipelineInterface<'a, B>>(
        &'a self,
        create_info: &GraphicsPipelineCreateInfo<'a, '_, B>,
    ) -> GraphicsPipeline<'a, B, TypedSignature<'a, B, P>>
    {


        let root_signature = self.renderer.get_cached_signature::<P>();

        let create_info_full = GraphicsPipelineCreateInfo {
            shader_stages: create_info.shader_stages,
            //vertex_input_state: &vertex_input_state,
            viewport_state: create_info.viewport_state,
            rasterization_state: create_info.rasterization_state,
            multisample_state: create_info.multisample_state,
            depth_stencil_state: create_info.depth_stencil_state,
            input_assembly_state: create_info.input_assembly_state,
            color_blend_state: create_info.color_blend_state,
            dynamic_state: DynamicStateFlags::empty(),
        };

        // validate the pipeline
        //let validation_result = validate_graphics(&create_info_full);
        //if let Err(e) = validation_result {
        //    panic!("graphics pipeline validation failed: {}", e);
        //}

        GraphicsPipeline {
            inner: unsafe {
                self.instance
                    .create_graphics_pipeline(&self.inner(),
                                              root_signature.0,
                                              P::SIGNATURE,
                                              &create_info_full)
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
    pub fn create_image(
        &self,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> Image<B> {
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

    /*
    pub fn create_pipeline_signature_typeless<'a, 'rr: 'a>(
        &'rr self,
        description: &'b PipelineSignatureDescription<'b>,
    ) -> PipelineSignatureTypeless<'a, B> {
        PipelineSignatureTypeless(unsafe {
            self.instance
                .create_pipeline_signature(&self.inner(), description)
        }, description)
    }*/

    pub fn create_arguments<'a, 'b, S: Signature<'a, B>>(
        &'a self,
        signature: S,
        inherited: impl IntoIterator<Item = BareArguments<'a, B>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, B>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferDescriptor<'a, 'b, B>>,
        index_buffer: Option<IndexBufferDescriptor<'a, B>>,
        render_targets: impl IntoIterator<Item = RenderTargetDescriptor<'a, B>>,
        depth_stencil_render_target: Option<RenderTargetDescriptor<'a, B>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = ScissorRect>,
    ) -> Arguments<'a, B, S> {
        Arguments {
            arguments: unsafe {
                self.instance.create_arguments(
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

    pub fn create_typed_arguments<'a, 'b, T: PipelineInterface<'a, B>>(
        &'a self,
        args: T,
    ) -> Arguments<'a, B, TypedSignature<'a, B, T::IntoInterface>> {
        let sig = self.renderer.get_cached_signature::<T::IntoInterface>();
        args.into_arguments(sig, self)
    }

    /*
    /// Creates a pipeline argument group.
    ///
    /// Note: this must be fast, and allocate as little as possible.
    /// Issue: this will entail a hash map lookup, though.
    /// Avoid intermediate buffers. Ideally, have the backend directly visit the arguments.
    pub fn create_arguments<'a, T: PipelineInterface<'a, B>>(
        &'a self,
        arguments: T,
    ) -> PipelineArguments<'a, B, T::IntoInterface> {
        // get the signature
        let signature = self.create_pipeline_signature_typeless(T::SIGNATURE);
        PipelineArguments(arguments.into_arguments(signature, self).0, PhantomData)
    }*/
}

//--------------------------------------------------------------------------------------------------

/// Renderer
pub struct Renderer<B: Backend> {
    instance: B::Instance,
    /// Arena for long-lived or cached objects, such as pipeline signatures.
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
    pub fn get_cached_signature<'r, P: PipelineInterface<'r, B>>(
        &'r self,
    ) -> TypedSignature<'r, B, P> {
        let typeid = TypeId::of::<P::UniqueType>();
        let cached = self.signature_cache.lock().unwrap().get(&typeid).cloned();
        if let Some(cached) = cached {
            unsafe { TypedSignature(&*cached, PhantomData) }
        } else {
            // signature not created yet
            let inherited = P::get_inherited_signatures(self);
            let sig = unsafe {self.instance.create_signature(
                self.default_arena.as_ref().unwrap(),
                &inherited,
                P::SIGNATURE,
            )};
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
    pub fn submit_frame(&self, command_buffers: Vec<CommandBuffer<'_, B>>) {
        let commands = sort_command_buffers(command_buffers);
        unsafe { self.instance.submit_frame(&commands) }
    }
}
