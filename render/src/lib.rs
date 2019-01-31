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

use crate::framebuffer::Framebuffer;
use crate::pipeline::build_vertex_input_interface;
//use crate::pipeline::validate::validate_graphics;
use crate::pipeline::DynamicStateFlags;
use crate::pipeline::GraphicsPipeline;
use crate::pipeline::GraphicsPipelineCreateInfo;
use crate::pipeline::GraphicsPipelineCreateInfoTypeless;
use crate::pipeline::GraphicsPipelineTypeless;
use crate::pipeline::PipelineInterface;
use crate::pipeline::ShaderModule;
use crate::pipeline::ShaderStageFlags;
use crate::pipeline::VertexInputState;
use std::mem;
use std::slice;
use crate::pipeline::PipelineArguments;
use crate::pipeline::Signature;

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

/// Trait implemented by renderer backends.
///
/// The `RendererBackend` trait provides an interface to create graphics resources and send commands
/// to one (TODO or more) GPU.
/// It has a number of associated types for various kinds of graphics objects.
/// It serves as an abstraction layer over a graphics API.
///
/// See the [autograph_render_gl] crate for an example implementation.
pub trait RendererBackend: Sync {
    // Some associated backend types (such as Framebuffers, or DescriptorSets) conceptually "borrow"
    // the referenced resources, and as such should have an associated lifetime parameter.
    // However, this cannot be expressed right now because of the lack of generic associated types
    // (a.k.a. associated type constructors, or ATCs).

    /// Creates a new empty Arena.
    fn create_arena(&self) -> Box<dyn traits::Arena>;

    /// Drops an arena and all the objects it owns.
    fn drop_arena(&self, arena: Box<dyn traits::Arena>);

    /// See [Renderer::create_swapchain](crate::Renderer::create_swapchain).
    fn create_swapchain<'a>(&self, arena: &'a dyn traits::Arena) -> &'a dyn traits::Swapchain;

    /// See [Renderer::default_swapchain](crate::Renderer::default_swapchain).
    fn default_swapchain<'a>(&'a self) -> Option<&'a dyn traits::Swapchain>;

    /// Creates an immutable image that cannot be modified by any operation (render, transfer, swaps or otherwise).
    /// Useful for long-lived texture data.
    fn create_immutable_image<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: &[u8],
    ) -> &'a dyn traits::Image;

    /// Creates an image containing uninitialized data.
    ///
    /// See [Arena::create_image](crate::arena::Arena::create_image).
    fn create_image<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a dyn traits::Image;

    /// Updates a region of an image.
    ///
    /// This function assumes that the format of data matches the internal format of the image.
    /// No conversion is performed.
    fn update_image(
        &self,
        image: &dyn traits::Image,
        min_extent: (u32, u32, u32),
        max_extent: (u32, u32, u32),
        data: &[u8],
    );

    /// See [Arena::create_framebuffer](crate::arena::Arena::create_framebuffer).
    fn create_framebuffer<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        color_attachments: &[&'a dyn traits::Image],
        depth_stencil_attachment: Option<&'a dyn traits::Image>,
    ) -> &'a dyn traits::Framebuffer;

    /// TODO
    fn create_immutable_buffer<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        size: u64,
        data: &[u8],
    ) -> &'a dyn traits::Buffer;

    /// TODO
    fn create_buffer<'a>(&self, arena: &'a dyn traits::Arena, size: u64) -> &'a dyn traits::Buffer;

    /// See [Arena::create_shader_module](crate::arena::Arena::create_shader_module).
    fn create_shader_module<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        data: &[u8],
        stage: ShaderStageFlags,
    ) -> &'a dyn traits::ShaderModule;

    /// See [Arena::create_graphics_pipeline](crate::arena::Arena::create_graphics_pipeline).
    fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        create_info: &GraphicsPipelineCreateInfoTypeless<'_, 'a>,
    ) -> &'a dyn traits::GraphicsPipeline;

    /// Creates a new pipeline argument group, which describes a set of resources to be bound to the graphics
    /// pipeline, and state to be set.
    fn create_pipeline_arguments<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        signature: &Signature,
    ) -> &'a dyn traits::PipelineArguments;

    /// Creates a reference to host data that is going to be used in pipeline arguments.
    fn create_host_reference<'a>(&self,
                                        arena: &'a dyn traits::Arena,
                                        data: &'a [u8]) -> &'a dyn traits::HostReference;

    /// Sends commands to the GPU for execution, and ends the current frame.
    /// Uploads all referenced host data to the GPU and releases the borrows.
    ///
    /// Precondition: the command list should be sorted by sortkey.
    fn submit_frame<'a>(&self, commands: &[Command<'a>]);
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
pub struct Arena<'rcx> {
    backend: &'rcx dyn RendererBackend,
    inner: Option<Box<dyn traits::Arena>>,
}

impl<'rcx> Drop for Arena<'rcx> {
    fn drop(&mut self) {
        self.backend.drop_arena(self.inner.take().unwrap())
    }
}

impl<'rcx> Arena<'rcx> {
    /// Returns the backend arena.
    pub fn inner_arena(&self) -> &dyn traits::Arena {
        self.inner.as_ref().unwrap().as_ref()
    }

    /// Creates a swapchain.
    #[inline]
    pub fn create_swapchain(&self) -> Swapchain {
        Swapchain(self.backend.create_swapchain(self.inner_arena()))
    }

    /// Creates a framebuffer.
    #[inline]
    pub fn create_framebuffer<'a>(
        &'a self,
        color_attachments: &[Image<'a>],
        depth_stencil_attachment: Option<Image<'a>>,
    ) -> Framebuffer<'a> {
        let raw_color_attachments = unsafe {
            slice::from_raw_parts(
                color_attachments.as_ptr() as *const &'a dyn traits::Image,
                color_attachments.len(),
            )
        };
        Framebuffer(self.backend.create_framebuffer(
            self.inner_arena(),
            raw_color_attachments,
            depth_stencil_attachment.map(|a| a.0),
        ))
    }

    /// Creates a shader module from SPIR-V bytecode.
    #[inline]
    pub fn create_shader_module<'a, 'spv>(
        &'a self,
        data: &'spv [u8],
        stage: ShaderStageFlags,
    ) -> ShaderModule<'a, 'spv> {
        ShaderModule(
            self.backend
                .create_shader_module(self.inner_arena(), data, stage),
            data,
        )
    }

    /// Creates a graphics pipeline given the pipeline description passed in create_info.
    #[inline]
    pub fn create_graphics_pipeline_typeless<'a>(
        &'a self,
        create_info: &GraphicsPipelineCreateInfoTypeless<'_, 'a>,
    ) -> GraphicsPipelineTypeless<'a> {
        GraphicsPipelineTypeless(
            self.backend
                .create_graphics_pipeline(self.inner_arena(), create_info),
        )
    }

    /// Creates a graphics pipeline given the pipeline description passed in create_info
    /// and information derived from the pipeline interface type.
    #[inline]
    pub fn create_graphics_pipeline<'a, Pipeline: PipelineInterface<'a>>(
        &'a self,
        create_info: &GraphicsPipelineCreateInfo<'_, 'a>,
        extra_signature: &Signature,
    ) -> GraphicsPipeline<'a, Pipeline> {
        // combine static & dynamic
        let vertex_layouts = <Pipeline as PipelineInterface<'a>>::SIGNATURE
            .vertex_layouts
            .iter()
            .cloned()
            .chain(extra_signature.vertex_layouts.iter().cloned())
            .collect::<Vec<_>>();
        let (vtx_input_bindings, vtx_input_attribs) = build_vertex_input_interface(&vertex_layouts);

        let vertex_input_state = VertexInputState {
            bindings: &vtx_input_bindings,
            attributes: &vtx_input_attribs,
        };

        let create_info_full = GraphicsPipelineCreateInfoTypeless {
            shader_stages: create_info.shader_stages,
            vertex_input_state: &vertex_input_state,
            viewport_state: create_info.viewport_state,
            rasterization_state: create_info.rasterization_state,
            multisample_state: create_info.multisample_state,
            depth_stencil_state: create_info.depth_stencil_state,
            input_assembly_state: create_info.input_assembly_state,
            color_blend_state: create_info.color_blend_state,
            dynamic_state: DynamicStateFlags::empty(),
            root_signature: <Pipeline as PipelineInterface<'a>>::SIGNATURE
        };

        // validate the pipeline
        //let validation_result = validate_graphics(&create_info_full);
        //if let Err(e) = validation_result {
        //    panic!("graphics pipeline validation failed: {}", e);
        //}

        GraphicsPipeline(
            self.backend
                .create_graphics_pipeline(self.inner_arena(), &create_info_full),
            PhantomData,
        )
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
    ) -> Image {
        Image(self.backend.create_immutable_image(
            self.inner_arena(),
            format,
            dimensions,
            mipcount,
            samples,
            usage,
            initial_data,
        ))
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
    ) -> Image {
        Image(self.backend.create_image(
            self.inner_arena(),
            scope,
            format,
            dimensions,
            mipcount,
            samples,
            usage,
        ))
    }

    /// Creates a GPU (device local) buffer.
    #[inline]
    pub fn create_buffer_typeless(&self, size: u64) -> BufferTypeless {
        BufferTypeless(self.backend.create_buffer(self.inner_arena(), size))
    }

    /// Creates a GPU (device local) buffer.
    #[inline]
    pub fn create_immutable_buffer_typeless(&self, size: u64, data: &[u8]) -> BufferTypeless {
        BufferTypeless(
            self.backend
                .create_immutable_buffer(self.inner_arena(), size, data),
        )
    }

    /// Creates an immutable, device-local GPU buffer containing an object of type T.
    #[inline]
    pub fn upload<T: Copy + 'static>(&self, data: &T) -> Buffer<T> {
        let size = mem::size_of::<T>();
        let bytes = unsafe { ::std::slice::from_raw_parts(data as *const T as *const u8, size) };

        Buffer(
            self.backend
                .create_immutable_buffer(self.inner_arena(), size as u64, bytes),
            PhantomData,
        )
    }

    /// Creates an immutable, device-local GPU buffer containing an array of objects of type T.
    #[inline]
    pub fn upload_slice<T: Copy + 'static>(&self, data: &[T]) -> Buffer<[T]> {
        let size = mem::size_of_val(data);
        let bytes = unsafe { ::std::slice::from_raw_parts(data.as_ptr() as *const u8, size) };

        Buffer(
            self.backend
                .create_immutable_buffer(self.inner_arena(), size as u64, bytes),
            PhantomData,
        )
    }

    /// Creates an immutable, device-local GPU buffer containing an array of objects of type T.
    #[inline]
    pub fn host_reference<'a, T: Copy + 'static>(&'a self, data: &'a T) -> HostReference<'a, T>
    {
        let size = mem::size_of::<T>();
        let bytes = unsafe { ::std::slice::from_raw_parts(data as *const T as *const u8, size) };

        HostReference(self.backend.create_host_reference(self.inner_arena(), bytes), PhantomData)
    }

    /// Creates an immutable, device-local GPU buffer containing an array of objects of type T.
    #[inline]
    pub fn host_slice<'a, T: Copy + 'static>(&'a self, data: &'a [T]) -> HostReference<'a, T> {

        let size = mem::size_of_val(data);
        let bytes = unsafe { ::std::slice::from_raw_parts(data.as_ptr() as *const u8, size) };

        HostReference(self.backend.create_host_reference(self.inner_arena(), bytes), PhantomData)
    }

    /// Creates a pipeline argument group.
    pub fn create_pipeline_arguments<'a, T: PipelineInterface<'a>>(
        &'a self,
        arguments: T,
    ) -> PipelineArguments<'a, T>
    {
        let args = self.backend.create_pipeline_arguments(
            self.inner_arena(),
            &<T as PipelineInterface<'a>>::SIGNATURE,
        );

        arguments.update_pipeline_arguments(self, args);

        PipelineArguments(
            args,
            PhantomData,
        )
    }
}

//--------------------------------------------------------------------------------------------------

/// Renderer
pub struct Renderer {
    backend: Box<dyn RendererBackend + 'static>,
}

impl Renderer {
    /// Creates a new renderer with the specified backend.
    pub fn new<R: RendererBackend + 'static>(backend: R) -> Renderer {
        Renderer {
            backend: Box::new(backend),
        }
    }

    pub fn create_arena(&self) -> Arena {
        Arena {
            backend: self.backend.as_ref(),
            inner: Some(self.backend.create_arena()),
        }
    }

    /// Returns the default swapchain if there is one.
    pub fn default_swapchain(&self) -> Option<Swapchain> {
        self.backend.default_swapchain().map(|s| Swapchain(s))
    }

    /// Creates a command buffer.
    pub fn create_command_buffer<'cmd>(&self) -> CommandBuffer<'cmd> {
        CommandBuffer::new()
    }

    /// Submits the given command buffers for rendering and ends the current frame.
    ///
    /// Frame-granularity synchronization points happen in this call.
    /// A new frame is implicitly started after this call.
    pub fn submit_frame(&self, command_buffers: Vec<CommandBuffer>) {
        let commands = sort_command_buffers(command_buffers);
        self.backend.submit_frame(&commands)
    }
}
