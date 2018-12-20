use super::{
    api as gl,
    api::types::*,
    descriptor::{DescriptorSet, DescriptorSetLayout},
    framebuffer::Framebuffer,
    image::{ImageDescription, RawImage},
    pipeline::GraphicsPipeline,
    pool::{BufferAliasKey, ImageAliasKey, ImagePool},
    shader::ShaderModule,
    sync::GpuSyncObject,
    upload::{MappedBuffer, UploadBuffer},
    util::SyncArena,
    Swapchain,
};
use fxhash::FxHashMap;
use gfx2::{
    AliasScope, Dimensions, Filter, Format, ImageUsageFlags, MipmapsCount, SamplerAddressMode,
    SamplerDescription, SamplerMipmapMode,
};
use slotmap;
use std::collections::VecDeque;

//--------------------------------------------------------------------------------------------------
fn min_filter_to_glenum(filter: Filter, mipmap_mode: SamplerMipmapMode) -> GLenum {
    match (filter, mipmap_mode) {
        (Filter::Nearest, SamplerMipmapMode::Linear) => gl::NEAREST_MIPMAP_LINEAR,
        (Filter::Linear, SamplerMipmapMode::Linear) => gl::LINEAR_MIPMAP_LINEAR,
        (Filter::Nearest, SamplerMipmapMode::Nearest) => gl::NEAREST_MIPMAP_NEAREST,
        (Filter::Linear, SamplerMipmapMode::Nearest) => gl::LINEAR_MIPMAP_NEAREST,
    }
}

fn mag_filter_to_glenum(filter: Filter) -> GLenum {
    match filter {
        Filter::Nearest => gl::NEAREST,
        Filter::Linear => gl::LINEAR,
    }
}

fn address_mode_to_glenum(mode: SamplerAddressMode) -> GLenum {
    match mode {
        SamplerAddressMode::Clamp => gl::CLAMP_TO_EDGE,
        SamplerAddressMode::Mirror => gl::MIRRORED_REPEAT,
        SamplerAddressMode::Wrap => gl::REPEAT,
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
pub struct AliasInfo<K: slotmap::Key> {
    pub key: K,
    pub scope: AliasScope,
}

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct Image {
    pub obj: GLuint,
    pub target: GLenum,
    pub should_destroy: bool,
    pub alias_info: Option<AliasInfo<ImageAliasKey>>,
}

#[derive(Debug)]
pub struct Buffer {
    pub obj: GLuint,
    pub should_destroy: bool,
    pub alias_info: Option<AliasInfo<BufferAliasKey>>,
    pub offset: usize,
    pub size: usize, // should be u64?
}

pub struct SamplerCache {
    // samplers are never deleted
    samplers: FxHashMap<SamplerDescription, GLuint>,
}

impl SamplerCache {
    pub fn new() -> SamplerCache {
        SamplerCache {
            samplers: FxHashMap::with_hasher(fxhash::FxBuildHasher::default()),
        }
    }

    pub fn get_sampler(&mut self, desc: &SamplerDescription) -> GLuint {
        *self.samplers.entry(desc.clone()).or_insert_with(|| unsafe {
            let mut obj = 0;
            gl::GenSamplers(1, &mut obj);
            gl::SamplerParameteri(
                obj,
                gl::TEXTURE_MIN_FILTER,
                min_filter_to_glenum(desc.min_filter, desc.mipmap_mode) as i32,
            );
            gl::SamplerParameteri(
                obj,
                gl::TEXTURE_MAG_FILTER,
                mag_filter_to_glenum(desc.mag_filter) as i32,
            );
            gl::SamplerParameteri(
                obj,
                gl::TEXTURE_WRAP_R,
                address_mode_to_glenum(desc.addr_u) as i32,
            );
            gl::SamplerParameteri(
                obj,
                gl::TEXTURE_WRAP_S,
                address_mode_to_glenum(desc.addr_v) as i32,
            );
            gl::SamplerParameteri(
                obj,
                gl::TEXTURE_WRAP_T,
                address_mode_to_glenum(desc.addr_w) as i32,
            );
            obj
        })
    }
}

//--------------------------------------------------------------------------------------------------
pub struct Arena {
    pub swapchains: SyncArena<Swapchain>,
    pub buffers: SyncArena<Buffer>,
    pub images: SyncArena<Image>,
    pub descriptor_sets: SyncArena<DescriptorSet>,
    pub descriptor_set_layouts: SyncArena<DescriptorSetLayout>,
    pub shader_modules: SyncArena<ShaderModule>,
    pub graphics_pipelines: SyncArena<GraphicsPipeline>,
    pub framebuffers: SyncArena<Framebuffer>,
    pub upload_buffer: UploadBuffer,
}

impl Arena {
    pub fn new(upload_buffer: UploadBuffer) -> Arena {
        Arena {
            swapchains: SyncArena::new(),
            buffers: SyncArena::new(),
            images: SyncArena::new(),
            descriptor_sets: SyncArena::new(),
            descriptor_set_layouts: SyncArena::new(),
            shader_modules: SyncArena::new(),
            graphics_pipelines: SyncArena::new(),
            framebuffers: SyncArena::new(),
            upload_buffer,
        }
    }
}

//--------------------------------------------------------------------------------------------------
pub struct Resources {
    image_pool: ImagePool,
    //buffer_pool: BufferPool,
    upload_buffer_size: usize,
    upload_buffers: Vec<MappedBuffer>,
    upload_buffers_in_use: VecDeque<GpuSyncObject<Vec<MappedBuffer>>>,
}

impl Resources {
    pub fn new(upload_buffer_size: usize) -> Resources {
        Resources {
            image_pool: ImagePool::new(),
            //buffer_pool: BufferPool::new(),
            upload_buffer_size,
            upload_buffers: Vec::new(),
            upload_buffers_in_use: VecDeque::new(),
        }
    }

    pub fn alloc_upload_buffer(&mut self) -> UploadBuffer {
        self.reclaim_upload_buffers();
        if self.upload_buffers.is_empty() {
            UploadBuffer::new(MappedBuffer::new(self.upload_buffer_size))
        } else {
            UploadBuffer::new(self.upload_buffers.pop().unwrap())
        }
    }

    pub fn reclaim_upload_buffers(&mut self) {
        while !self.upload_buffers_in_use.is_empty() {
            let ready = self.upload_buffers_in_use.front().unwrap().try_wait();
            if ready.is_ok() {
                let buffers = self.upload_buffers_in_use.pop_front().unwrap();
                let mut buffers = unsafe { buffers.into_inner_unsynchronized() };
                self.upload_buffers.append(&mut buffers);
            } else {
                break;
            }
        }
    }

    pub fn create_arena(&mut self) -> Arena {
        Arena::new(self.alloc_upload_buffer())
    }

    // arena can't drop before commands that refer to the objects inside are submitted
    pub fn drop_arena(&mut self, arena: Arena)
    where
        Self: Sized,
    {
        // recover resources
        arena.images.into_vec().into_iter().for_each(|image| {
            if image.should_destroy {
                RawImage {
                    obj: image.obj,
                    target: image.target,
                }
                .destroy()
            } else {
                if let Some(ref alias_info) = image.alias_info {
                    self.image_pool
                        .destroy(alias_info.key, alias_info.scope, |image| {
                            image.destroy();
                        });
                } else {
                    // not owned, and not in a pool: maybe an alias or an image view?
                }
            }
        });

        self.upload_buffers_in_use
            .push_back(GpuSyncObject::new(vec![arena.upload_buffer.into_inner()]));
    }

    //----------------------------------------------------------------------------------------------
    pub fn alloc_aliased_image<'a>(
        &mut self,
        arena: &'a Arena,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a Image {
        let desc = ImageDescription::new(format, dimensions, mipcount, samples, usage);
        let (key, raw_img) = self.image_pool.alloc(scope, desc, |d| {
            debug!(
                "Allocating new scoped image {:?} ({:?}, {:?}, mips: {}, samples: {})",
                d.dimensions, d.format, d.usage, d.mipcount, d.samples
            );
            if d.usage
                .intersects(ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLED)
            {
                // will be used as storage or sampled image
                RawImage::new_texture(
                    d.format,
                    &d.dimensions,
                    MipmapsCount::Specific(d.mipcount),
                    samples,
                )
            } else {
                // only used as color attachments: can use a renderbuffer instead
                RawImage::new_renderbuffer(d.format, &d.dimensions, d.samples)
            }
        });

        arena.images.alloc(Image {
            alias_info: AliasInfo { key, scope }.into(),
            obj: raw_img.obj,
            target: raw_img.target,
            should_destroy: false,
        })
    }
}
