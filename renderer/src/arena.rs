use crate::buffer::BufferData;
use crate::descriptor::Descriptor;
use crate::descriptor::DescriptorSetLayoutBinding;
use crate::format::Format;
use crate::image::Dimensions;
use crate::image::ImageUsageFlags;
use crate::image::MipmapsCount;
use crate::image::SamplerDescription;
use crate::interface::DescriptorSetInterface;
use crate::interface::DescriptorSetInterfaceVisitor;
use crate::pipeline::GraphicsPipelineCreateInfo;
use crate::shader::ShaderStageFlags;
use crate::traits;
use crate::AliasScope;
use crate::Renderer;
use crate::RendererBackend;
use derivative::Derivative;
use std::marker::PhantomData;
use std::mem;

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct Buffer<'a, R: RendererBackend, T: BufferData + ?Sized>(
    pub &'a R::Buffer,
    pub(crate) PhantomData<T>,
);

impl<'a, R: RendererBackend, T: BufferData + ?Sized> Buffer<'a, R, T> {
    pub fn byte_size(&self) -> u64 {
        traits::Buffer::size(self.0)
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct BufferTypeless<'a, R: RendererBackend>(pub &'a R::Buffer);

impl<'a, R: RendererBackend> BufferTypeless<'a, R> {
    pub fn byte_size(&self) -> u64 {
        traits::Buffer::size(self.0)
    }
}
impl<'a, R: RendererBackend, T: BufferData + ?Sized> From<Buffer<'a, R, T>>
    for BufferTypeless<'a, R>
{
    fn from(from: Buffer<'a, R, T>) -> Self {
        BufferTypeless(from.0)
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct DescriptorSetLayout<'a, R: RendererBackend>(pub &'a R::DescriptorSetLayout);

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct DescriptorSet<'a, R: RendererBackend>(pub &'a R::DescriptorSet);

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct Image<'a, R: RendererBackend>(pub &'a R::Image);

impl<'a, R: RendererBackend> Image<'a, R> {
    pub fn into_sampled(self, d: SamplerDescription) -> SampledImage<'a, R> {
        SampledImage(self.0, d)
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct SampledImage<'a, R: RendererBackend>(pub &'a R::Image, pub SamplerDescription);

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct ShaderModule<'a, R: RendererBackend>(pub &'a R::ShaderModule);

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct GraphicsPipeline<'a, R: RendererBackend>(pub &'a R::GraphicsPipeline);

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct Framebuffer<'a, R: RendererBackend>(pub &'a R::Framebuffer);

//--------------------------------------------------------------------------------------------------
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""), Debug(bound = ""))]
pub struct Swapchain<'a, R: RendererBackend>(pub &'a R::Swapchain);

impl<'a, R: RendererBackend> Swapchain<'a, R> {
    pub fn size(&self) -> (u32, u32) {
        traits::Swapchain::size(self.0)
    }
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
    pub fn create_swapchain(&self) -> Swapchain<R> {
        Swapchain(self.backend.create_swapchain(self.inner_arena()))
    }

    /// Creates a framebuffer.
    #[inline]
    pub fn create_framebuffer<'a>(
        &'a self,
        color_attachments: &[Image<'a, R>],
        depth_stencil_attachment: Option<Image<'a, R>>,
    ) -> Framebuffer<'a, R> {
        Framebuffer(self.backend.create_framebuffer(
            self.inner_arena(),
            color_attachments,
            depth_stencil_attachment,
        ))
    }

    /// Creates a shader module.
    #[inline]
    pub fn create_shader_module(&self, data: &[u8], stage: ShaderStageFlags) -> ShaderModule<R> {
        ShaderModule(
            self.backend
                .create_shader_module(self.inner_arena(), data, stage),
        )
    }

    /// Creates a graphics pipeline.
    /// Pipeline = all shaders + input layout + output layout (expected buffers)
    /// Creation process?
    #[inline]
    pub fn create_graphics_pipeline<'a>(
        &'a self,
        create_info: &GraphicsPipelineCreateInfo<'_, 'a, R>,
    ) -> GraphicsPipeline<'a, R> {
        GraphicsPipeline(
            self.backend
                .create_graphics_pipeline(self.inner_arena(), create_info),
        )
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
    ) -> Image<R> {
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
    ) -> Image<R> {
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
    pub fn create_buffer_typeless(&self, size: u64) -> BufferTypeless<R> {
        BufferTypeless(self.backend.create_buffer(self.inner_arena(), size))
    }

    /// Creates a GPU (device local) buffer.
    #[inline]
    pub fn create_immutable_buffer_typeless(&self, size: u64, data: &[u8]) -> BufferTypeless<R> {
        BufferTypeless(
            self.backend
                .create_immutable_buffer(self.inner_arena(), size, data),
        )
    }

    /// Creates a GPU (device local) buffer.
    #[inline]
    pub fn upload<T: Copy + 'static>(&self, data: &T) -> Buffer<R, T> {
        let size = mem::size_of::<T>();
        let bytes = unsafe { ::std::slice::from_raw_parts(data as *const T as *const u8, size) };

        Buffer(
            self.backend
                .create_immutable_buffer(self.inner_arena(), size as u64, bytes),
            PhantomData,
        )
    }

    #[inline]
    pub fn upload_slice<T: Copy + 'static>(&self, data: &[T]) -> Buffer<R, [T]> {
        let size = mem::size_of_val(data);
        let bytes = unsafe { ::std::slice::from_raw_parts(data.as_ptr() as *const u8, size) };

        Buffer(
            self.backend
                .create_immutable_buffer(self.inner_arena(), size as u64, bytes),
            PhantomData,
        )
    }

    #[inline]
    pub fn create_descriptor_set_layout<'a>(
        &'a self,
        bindings: &[DescriptorSetLayoutBinding],
    ) -> DescriptorSetLayout<'a, R> {
        DescriptorSetLayout(
            self.backend
                .create_descriptor_set_layout(self.inner_arena(), bindings),
        )
    }

    pub fn create_descriptor_set<'a>(
        &'a self,
        layout: DescriptorSetLayout<'a, R>,
        interface: impl DescriptorSetInterface<'a, R>,
    ) -> DescriptorSet<'a, R> {
        struct Visitor<'a, R: RendererBackend> {
            descriptors: Vec<Descriptor<'a, R>>,
        }

        impl<'a, R: RendererBackend> DescriptorSetInterfaceVisitor<'a, R> for Visitor<'a, R> {
            fn visit_buffer(
                &mut self,
                _binding: u32,
                buffer: BufferTypeless<'a, R>,
                offset: usize,
                size: usize,
            ) {
                self.descriptors.push(Descriptor::Buffer {
                    buffer,
                    offset,
                    size,
                })
            }

            fn visit_sampled_image(
                &mut self,
                _binding: u32,
                image: Image<'a, R>,
                sampler: &SamplerDescription,
            ) {
                self.descriptors.push(Descriptor::SampledImage {
                    sampler: *sampler,
                    img: image,
                })
            }
        }

        let mut visitor = Visitor {
            descriptors: Vec::new(),
        };

        interface.do_visit(&mut visitor);

        DescriptorSet(self.backend.create_descriptor_set(
            self.inner_arena(),
            layout.0,
            &visitor.descriptors,
        ))
    }
}

impl<R: RendererBackend> Renderer<R> {
    pub fn create_arena(&self) -> Arena<R> {
        Arena {
            backend: &self.backend,
            inner_arena: Some(self.backend.create_arena()),
        }
    }
}