use crate::format::Format;
use crate::image::Dimensions;
use crate::image::MipmapsCount;
use crate::shader::ShaderStageFlags;
use crate::AliasScope;
use crate::Command;
use crate::Descriptor;
use crate::GraphicsPipelineCreateInfo;
use crate::ImageUsageFlags;
use std::fmt::Debug;

//--------------------------------------------------------------------------------------------------

/// Trait implemented by backend swapchain objects.
pub trait Swapchain: Debug {
    fn size(&self) -> (u32, u32);
}
/// Trait implemented by backend buffer objects.
pub trait Buffer: Debug {
    fn size(&self) -> u64;
}
/// Trait implemented by backend image objects.
pub trait Image: Debug {}
/// Trait implemented by backend framebuffer objects.
pub trait Framebuffer: Debug {}
/// Trait implemented by backend descriptor set layout objects.
pub trait DescriptorSetLayout: Debug {}
/// Trait implemented by backend shader module objects.
pub trait ShaderModule: Debug {}
/// Trait implemented by backend graphics pipeline objects.
pub trait GraphicsPipeline: Debug {}
/// Trait implemented by backend descriptor set objects.
pub trait DescriptorSet: Debug {}

/// Trait implemented by renderer backends.
///
/// The `RendererBackend` trait provides an interface to create graphics resources and send commands
/// to one (TODO or more) GPU.
/// It has a number of associated types for various kinds of graphics objects.
/// It serves as an abstraction layer over a graphics API.
///
/// See the [gfx2_backend_gl] crate for an example implementation.
pub trait RendererBackend: Sync {
    // Some associated backend types (such as Framebuffers, or DescriptorSets) conceptually "borrow"
    // the referenced resources, and as such should have an associated lifetime parameter.
    // However, this cannot be expressed right now because of the lack of generic associated types
    // (a.k.a. associated type constructors, or ATCs).

    // XXX the 'static bounds may not be necessary: I put them to avoid specifying complex bounds
    // in other areas of the library.
    // That said, without ATCs, the associated types can't
    // really be bounded by anything other than 'static.
    // They don't need to be sized, however, as all we do is take references to them.

    /// Backend object type for swapchains
    type Swapchain: ?Sized + Swapchain + 'static;
    /// Backend object type for framebuffers
    type Framebuffer: ?Sized + Framebuffer + 'static;
    /// Backend object type for buffers
    type Buffer: ?Sized + Buffer + 'static;
    /// Backend object type for images
    type Image: ?Sized + Image + 'static;
    /// Backend object type for descriptor sets
    type DescriptorSet: ?Sized + DescriptorSet + 'static;
    /// Backend object type for descriptor set layouts
    type DescriptorSetLayout: ?Sized + DescriptorSetLayout + 'static;
    /// Backend object type for shader modules
    type ShaderModule: ?Sized + ShaderModule + 'static;
    /// Backend object type for graphics pipelines
    type GraphicsPipeline: ?Sized + GraphicsPipeline + 'static;

    /// Backend arena type.
    ///
    /// Arenas own the resources created by the backend, and are the primary
    /// way of managing the lifetime of backend resources.
    /// Resources contained in arenas are dropped all at once.
    /// This type is not used directly: see the [Arena](crate::arena::Arena) wrapper.
    type Arena: Sync;

    /// Creates a new empty Arena.
    fn create_arena(&self) -> Self::Arena;

    /// Drops an arena and all the objects it owns.
    fn drop_arena(&self, arena: Self::Arena)
    where
        Self: Sized;

    /// See [Renderer::create_swapchain](crate::Renderer::create_swapchain).
    fn create_swapchain<'a>(&self, arena: &'a Self::Arena) -> &'a Self::Swapchain
    where
        Self: Sized;

    /// See [Renderer::default_swapchain](crate::Renderer::default_swapchain).
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
    ///
    /// See [Arena::create_image](crate::arena::Arena::create_image).
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

    /// Updates a region of an image.
    ///
    /// This function assumes that the format of data matches the internal format of the image.
    /// No conversion is performed.
    fn update_image(
        &self,
        image: &Self::Image,
        min_extent: (u32,u32,u32),
        max_extent: (u32,u32,u32),
        data: &[u8]
        ) where Self: Sized;

    /// See [Arena::create_framebuffer](crate::arena::Arena::create_framebuffer).
    fn create_framebuffer<'a>(
        &self,
        arena: &'a Self::Arena,
        color_attachments: &[crate::Image<'a, Self>],
        depth_stencil_attachment: Option<crate::Image<'a, Self>>,
    ) -> &'a Self::Framebuffer
    where
        Self: Sized;

    /// TODO
    fn create_immutable_buffer<'a>(
        &self,
        arena: &'a Self::Arena,
        size: u64,
        data: &[u8],
    ) -> &'a Self::Buffer
    where
        Self: Sized;

    /// TODO
    fn create_buffer<'a>(&self, arena: &'a Self::Arena, size: u64) -> &'a Self::Buffer
    where
        Self: Sized;

    /// See [Arena::create_shader_module](crate::arena::Arena::create_shader_module).
    fn create_shader_module<'a>(
        &self,
        arena: &'a Self::Arena,
        data: &[u8],
        stage: ShaderStageFlags,
    ) -> &'a Self::ShaderModule
    where
        Self: Sized;

    /// See [Arena::create_graphics_pipeline](crate::arena::Arena::create_graphics_pipeline).
    fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a Self::Arena,
        create_info: &GraphicsPipelineCreateInfo<'_, 'a, Self>,
    ) -> &'a Self::GraphicsPipeline
    where
        Self: Sized;

    /// Creates a descriptor set layout, describing the resources and binding points expected
    /// by a shader.
    fn create_descriptor_set_layout<'a>(
        &self,
        arena: &'a Self::Arena,
        bindings: &[crate::DescriptorSetLayoutBinding<'_>],
    ) -> &'a Self::DescriptorSetLayout
    where
        Self: Sized;

    /// Creates a new descriptor set, which describes a set of resources to be bound to the graphics
    /// pipeline.
    fn create_descriptor_set<'a>(
        &self,
        arena: &'a Self::Arena,
        layout: &Self::DescriptorSetLayout,
        descriptors: &[Descriptor<'a, Self>],
    ) -> &'a Self::DescriptorSet
    where
        Self: Sized;

    /// Sends commands to the GPU for execution, and ends the current frame.
    ///
    /// Precondition: the command list should be sorted by sortkey.
    fn submit_frame<'a>(&self, commands: &[Command<'a, Self>])
    where
        Self: Sized;
}

/*
//--------------------------------------------------------------------------------------------------
struct ArenaAny(Box<Any>);

//--------------------------------------------------------------------------------------------------
trait RendererBackendAny {
    fn create_arena(&self) -> ArenaAny;
    fn drop_arena(&self, arena: ArenaAny);
    fn create_swapchain<'a>(&self, arena: &'a ArenaAny) -> &'a dyn SwapchainBackend;
    fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx dyn SwapchainBackend>;
    fn create_immutable_image<'a>(
        &self,
        arena: &'a ArenaAny,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: &[u8],
    ) -> &'a dyn ImageBackend;
    fn create_image<'a>(
        &self,
        arena: &'a ArenaAny,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a dyn ImageBackend;
    fn create_framebuffer<'a>(
        &self,
        arena: &'a ArenaAny,
        color_attachments: &[Image<'a, dyn RendererBackendAny>],
        depth_stencil_attachment: Option<Image<'a, dyn RendererBackendAny>>,
    ) -> &'a dyn FramebufferBackend;
    fn create_immutable_buffer<'a>(
        &self,
        arena: &'a ArenaAny,
        size: u64,
        data: &[u8],
    ) -> &'a dyn BufferBackend;
    fn create_buffer<'a>(&self, arena: &'a ArenaAny, size: u64) -> &'a dyn BufferBackend;
    fn create_shader_module<'a>(
        &self,
        arena: &'a ArenaAny,
        data: &[u8],
        stage: ShaderStageFlags,
    ) -> &'a dyn ShaderModuleBackend;
    fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a ArenaAny,
        create_info: &GraphicsPipelineCreateInfo<'_, 'a, dyn RendererBackendAny>,
    ) -> &'a dyn GraphicsPipelineBackend;
    fn create_descriptor_set_layout<'a>(
        &self,
        arena: &'a ArenaAny,
        bindings: &[DescriptorSetLayoutBinding<'_>],
    ) -> &'a dyn DescriptorSetLayoutBackend;
    fn create_descriptor_set<'a>(
        &self,
        arena: &'a ArenaAny,
        layout: &dyn DescriptorSetLayoutBackend,
        descriptors: &[Descriptor<'a, dyn RendererBackendAny>],
    ) -> &'a dyn DescriptorSetBackend;
    fn submit_frame<'a>(&self, commands: &[Command<'a, dyn RendererBackendAny>]);
}

impl RendererBackend for dyn RendererBackendAny {
    type Swapchain = dyn SwapchainBackend;
    type Framebuffer = dyn FramebufferBackend;
    type Buffer = dyn BufferBackend;
    type Image = dyn ImageBackend;
    type DescriptorSet = dyn DescriptorSetBackend;
    type DescriptorSetLayout = dyn DescriptorSetLayoutBackend;
    type ShaderModule = dyn ShaderModuleBackend;
    type GraphicsPipeline = dyn GraphicsPipelineBackend;

    type Arena = ArenaAny;

    fn create_arena(&self) -> <Self as RendererBackend>::Arena {
        unimplemented!()
    }

    fn drop_arena(&self, arena: <Self as RendererBackend>::Arena)
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_swapchain<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
    ) -> &'a <Self as RendererBackend>::Swapchain
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx <Self as RendererBackend>::Swapchain> {
        unimplemented!()
    }

    fn create_immutable_image<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: _,
        initial_data: &[u8],
    ) -> &'a <Self as RendererBackend>::Image
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_image<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: _,
    ) -> &'a <Self as RendererBackend>::Image
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_framebuffer<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        color_attachments: &[Image<'a, Self>],
        depth_stencil_attachment: Option<Image<'a, Self>>,
    ) -> &'a <Self as RendererBackend>::Framebuffer
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_immutable_buffer<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        size: u64,
        data: &[u8],
    ) -> &'a <Self as RendererBackend>::Buffer
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_buffer<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        size: u64,
    ) -> &'a <Self as RendererBackend>::Buffer
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_shader_module<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        data: &[u8],
        stage: _,
    ) -> &'a <Self as RendererBackend>::ShaderModule
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        create_info: &GraphicsPipelineCreateInfo<'_, 'a, Self>,
    ) -> &'a <Self as RendererBackend>::GraphicsPipeline
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_descriptor_set_layout<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        bindings: &[DescriptorSetLayoutBinding<'_>],
    ) -> &'a <Self as RendererBackend>::DescriptorSetLayout
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn create_descriptor_set<'a>(
        &self,
        arena: &'a <Self as RendererBackend>::Arena,
        layout: &<Self as RendererBackend>::DescriptorSetLayout,
        descriptors: &[Descriptor<'a, Self>],
    ) -> &'a <Self as RendererBackend>::DescriptorSet
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn submit_frame<'a>(&self, commands: &[Command<'a, Self>])
    where
        Self: Sized,
    {
        unimplemented!()
    }
}*/
