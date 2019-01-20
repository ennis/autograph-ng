use crate::aliaspool::AliasPool;
use crate::api as gl;
use crate::api::types::*;
use crate::api::Gl;
use crate::buffer::GlBuffer;
use crate::buffer::MappedBuffer;
use crate::buffer::RawBuffer;
use crate::buffer::UploadBuffer;
use crate::command::StateCache;
use crate::command::SubmissionContext;
use crate::descriptor::GlDescriptorSet;
use crate::descriptor::GlDescriptorSetLayout;
use crate::framebuffer::GlFramebuffer;
use crate::image::upload_image_region;
use crate::image::GlImage;
use crate::image::ImageAliasKey;
use crate::image::ImageDescription;
use crate::image::RawImage;
use crate::pipeline::create_graphics_pipeline_internal;
use crate::pipeline::GlGraphicsPipeline;
use crate::pipeline::GlShaderModule;
use crate::sampler::SamplerCache;
use crate::swapchain::GlSwapchain;
use crate::swapchain::SwapchainInner;
use crate::sync::GpuSyncObject;
use crate::sync::Timeline;
use crate::util::SyncArena;
use crate::util::SyncArenaHashMap;
use crate::AliasInfo;
use crate::DowncastPanic;
use crate::ImplementationParameters;
use autograph_render::command::Command;
use autograph_render::descriptor::Descriptor;
use autograph_render::descriptor::DescriptorSetLayoutBinding;
use autograph_render::format::Format;
use autograph_render::image::Dimensions;
use autograph_render::image::ImageUsageFlags;
use autograph_render::image::MipmapsCount;
use autograph_render::pipeline::GraphicsPipelineCreateInfoTypeless;
use autograph_render::pipeline::ShaderStageFlags;
use autograph_render::traits;
use autograph_render::AliasScope;
use autograph_render::RendererBackend;
use config::Config;
use std::any::TypeId;
use std::collections::VecDeque;
use std::ffi::CStr;
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::str;
use std::sync::Mutex;
use std::time::Duration;

//--------------------------------------------------------------------------------------------------
extern "system" fn debug_callback(
    _source: GLenum,
    _ty: GLenum,
    _id: GLuint,
    severity: GLenum,
    length: GLsizei,
    msg: *const GLchar,
    _data: *mut GLvoid,
) {
    let str = unsafe {
        str::from_utf8(slice::from_raw_parts(msg as *const u8, length as usize)).unwrap()
    };
    let level = match severity {
        gl::DEBUG_SEVERITY_HIGH => log::Level::Error,
        gl::DEBUG_SEVERITY_MEDIUM => log::Level::Warn,
        gl::DEBUG_SEVERITY_LOW => log::Level::Info,
        gl::DEBUG_SEVERITY_NOTIFICATION => log::Level::Debug,
        _ => log::Level::Debug,
    };
    log!(level, "(GL) {}", str);
}

//--------------------------------------------------------------------------------------------------
pub(crate) struct GlArena {
    pub(crate) _swapchains: SyncArena<GlSwapchain>,
    pub(crate) buffers: SyncArena<GlBuffer>,
    pub(crate) images: SyncArena<GlImage>,
    pub(crate) descriptor_sets: SyncArena<GlDescriptorSet>,
    pub(crate) descriptor_set_layouts: SyncArena<GlDescriptorSetLayout>,
    pub(crate) shader_modules: SyncArena<GlShaderModule>,
    pub(crate) graphics_pipelines: SyncArena<GlGraphicsPipeline>,
    pub(crate) framebuffers: SyncArena<GlFramebuffer>,
    pub(crate) upload_buffer: UploadBuffer,
}

impl GlArena {
    pub(crate) fn new(upload_buffer: UploadBuffer) -> GlArena {
        GlArena {
            _swapchains: SyncArena::new(),
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

impl traits::Arena for GlArena {}

//--------------------------------------------------------------------------------------------------
pub(crate) type ImagePool = AliasPool<ImageDescription, ImageAliasKey, RawImage>;
//pub(crate) type BufferPool = AliasPool<BufferDescription, BufferAliasKey, RawBuffer>;

///
struct Resources {
    image_pool: ImagePool,
    //buffer_pool: BufferPool,
    upload_buffer_size: usize,
    upload_buffers: Vec<MappedBuffer>,
    upload_buffers_in_use: VecDeque<GpuSyncObject<Vec<MappedBuffer>>>,
}

impl Resources {
    fn new(upload_buffer_size: usize) -> Resources {
        Resources {
            image_pool: ImagePool::new(),
            //buffer_pool: BufferPool::new(),
            upload_buffer_size,
            upload_buffers: Vec::new(),
            upload_buffers_in_use: VecDeque::new(),
        }
    }

    fn alloc_upload_buffer(&mut self, gl: &Gl) -> UploadBuffer {
        self.reclaim_upload_buffers(gl);
        if self.upload_buffers.is_empty() {
            UploadBuffer::new(MappedBuffer::new(gl, self.upload_buffer_size))
        } else {
            UploadBuffer::new(self.upload_buffers.pop().unwrap())
        }
    }

    fn reclaim_upload_buffers(&mut self, gl: &Gl) {
        while !self.upload_buffers_in_use.is_empty() {
            let ready = self.upload_buffers_in_use.front().unwrap().try_wait(gl);
            if ready.is_ok() {
                let buffers = self.upload_buffers_in_use.pop_front().unwrap();
                let mut buffers = unsafe { buffers.into_inner_unsynchronized(gl) };
                self.upload_buffers.append(&mut buffers);
            } else {
                break;
            }
        }
    }

    fn create_arena(&mut self, gl: &Gl) -> Box<GlArena> {
        Box::new(GlArena::new(self.alloc_upload_buffer(gl)))
    }

    // arena can't drop before commands that refer to the objects inside are submitted
    fn drop_arena(&mut self, gl: &Gl, arena: Box<GlArena>)
    where
        Self: Sized,
    {
        // recover resources
        arena.images.into_vec().into_iter().for_each(|image| {
            if image.should_destroy {
                image.raw.destroy(gl)
            } else {
                if let Some(ref alias_info) = image.alias_info {
                    self.image_pool
                        .destroy(alias_info.key, alias_info.scope, |image| {
                            image.destroy(gl);
                        });
                } else {
                    // not owned, and not in a pool: maybe an alias or an image view?
                }
            }
        });

        arena.buffers.into_vec().into_iter().for_each(|buf| {
            if buf.should_destroy {
                buf.raw.destroy(gl)
            }
        });

        arena.framebuffers.into_vec().into_iter().for_each(|fb| {
            fb.destroy(gl);
        });

        self.upload_buffers_in_use.push_back(GpuSyncObject::new(
            gl,
            vec![arena.upload_buffer.into_inner()],
        ));
    }

    //----------------------------------------------------------------------------------------------
    fn alloc_aliased_image<'a>(
        &mut self,
        gl: &Gl,
        arena: &'a GlArena,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a GlImage {
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
                    gl,
                    d.format,
                    &d.dimensions,
                    MipmapsCount::Specific(d.mipcount),
                    samples,
                )
            } else {
                // only used as color attachments: can use a renderbuffer instead
                RawImage::new_renderbuffer(gl, d.format, &d.dimensions, d.samples)
            }
        });

        arena.images.alloc(GlImage {
            alias_info: AliasInfo { key, scope }.into(),
            raw: raw_img.clone(),
            should_destroy: false,
        })
    }
}

//--------------------------------------------------------------------------------------------------
pub struct OpenGlBackend {
    rsrc: Mutex<Resources>,
    timeline: Mutex<Timeline>,
    frame_num: Mutex<u64>, // replace with AtomicU64 once stabilized
    state_cache: Mutex<StateCache>,
    sampler_cache: Mutex<SamplerCache>,
    desc_set_layout_cache: SyncArenaHashMap<TypeId, GlDescriptorSetLayout>,
    limits: ImplementationParameters,
    //window: GlWindow,
    def_swapchain: GlSwapchain,
    max_frames_in_flight: u32,
    gl: gl::Gl,
}

impl OpenGlBackend {
    pub fn with_gl(
        cfg: &Config,
        gl: gl::Gl,
        default_swapchain: Box<dyn SwapchainInner>,
    ) -> OpenGlBackend {
        unsafe {
            gl.Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl.DebugMessageCallback(debug_callback as GLDEBUGPROC, ptr::null());
            gl.DebugMessageControl(
                gl::DONT_CARE,
                gl::DONT_CARE,
                gl::DONT_CARE,
                0,
                ptr::null(),
                1,
            );

            let mut major_version = mem::uninitialized();
            let mut minor_version = mem::uninitialized();
            gl.GetIntegerv(gl::MAJOR_VERSION, &mut major_version);
            gl.GetIntegerv(gl::MINOR_VERSION, &mut minor_version);

            let vendor = CStr::from_ptr(gl.GetString(gl::VENDOR) as *const c_char);
            let renderer = CStr::from_ptr(gl.GetString(gl::RENDERER) as *const c_char);

            debug!(
                "OpenGL version {}.{} (vendor: {:?}, renderer: {:?})",
                major_version, minor_version, vendor, renderer
            );
        }

        let upload_buffer_size = cfg
            .get::<u64>("gfx.default_upload_buffer_size")
            .unwrap_or(4 * 1024 * 1024);
        assert!(upload_buffer_size <= usize::max_value() as u64);
        let max_frames_in_flight = cfg.get::<u32>("gfx.max_frames_in_flight").unwrap_or(2);

        let timeline = Timeline::new(0);

        let limits = ImplementationParameters::populate(&gl);
        let state_cache = StateCache::new(&limits);

        OpenGlBackend {
            rsrc: Mutex::new(Resources::new(upload_buffer_size as usize)),
            timeline: Mutex::new(timeline),
            frame_num: Mutex::new(1),
            def_swapchain: GlSwapchain {
                inner: default_swapchain,
            },
            desc_set_layout_cache: SyncArenaHashMap::new(),
            gl,
            max_frames_in_flight,
            limits,
            state_cache: Mutex::new(state_cache),
            sampler_cache: Mutex::new(SamplerCache::new()),
        }
    }

    /// Creates a new OpenGlBackend from the current OpenGL context.
    ///
    /// Panics if no context is currently bound, or if the current context does not
    /// satisfy the minimum requirements of the backend implementation.
    ///
    pub fn from_current_context() -> OpenGlBackend {
        // get version, check 4.6, or DSA + SPIR-V

        unimplemented!()
    }
}

// TODO move this into a function in the spirv module
const SPIRV_MAGIC: u32 = 0x0723_0203;
const UPLOAD_DEDICATED_THRESHOLD: usize = 65536;
const FRAME_WAIT_TIMEOUT: Duration = Duration::from_millis(500);

impl RendererBackend for OpenGlBackend {
    fn create_arena(&self) -> Box<dyn traits::Arena> {
        self.rsrc.lock().unwrap().create_arena(&self.gl)
    }

    fn drop_arena(&self, arena: Box<dyn traits::Arena>) {
        let arena: Box<GlArena> = arena.downcast_unwrap();
        self.rsrc.lock().unwrap().drop_arena(&self.gl, arena)
    }

    //----------------------------------------------------------------------------------------------
    fn create_swapchain<'a>(&self, _arena: &'a dyn traits::Arena) -> &'a dyn traits::Swapchain {
        unimplemented!()
    }

    fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx dyn traits::Swapchain> {
        Some(&self.def_swapchain)
    }

    //----------------------------------------------------------------------------------------------
    fn create_immutable_image<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        fmt: Format,
        dims: Dimensions,
        mips: MipmapsCount,
        samples: u32,
        _usage: ImageUsageFlags,
        data: &[u8],
    ) -> &'a dyn traits::Image {
        let arena: &GlArena = arena.downcast_ref_unwrap();
        // initial data specified, allocate a texture
        let raw = RawImage::new_texture(&self.gl, fmt, &dims, mips, samples);
        unsafe {
            upload_image_region(
                &self.gl,
                raw.target,
                raw.obj,
                fmt,
                0,
                (0, 0, 0),
                dims.width_height_depth(),
                data,
            );
        }

        arena.images.alloc(GlImage {
            should_destroy: true,
            raw,
            alias_info: None,
        })
    }

    //----------------------------------------------------------------------------------------------
    fn create_image<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        scope: AliasScope,
        fmt: Format,
        dims: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a dyn traits::Image {
        let arena: &GlArena = arena.downcast_ref_unwrap();
        self.rsrc
            .lock()
            .unwrap()
            .alloc_aliased_image(&self.gl, arena, scope, fmt, dims, mipcount, samples, usage)
    }

    //----------------------------------------------------------------------------------------------

    /// Creates a framebuffer. See trait documentation for explanation of unsafety.
    fn create_framebuffer<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        color_att: &[&'a dyn traits::Image],
        depth_stencil_att: Option<&'a dyn traits::Image>,
    ) -> &'a dyn traits::Framebuffer {
        let arena: &GlArena = arena.downcast_ref_unwrap();
        arena
            .framebuffers
            .alloc(GlFramebuffer::new(&self.gl, color_att, depth_stencil_att).unwrap())
    }

    //----------------------------------------------------------------------------------------------
    fn create_immutable_buffer<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        size: u64,
        data: &[u8],
    ) -> &'a dyn traits::Buffer {
        let arena: &GlArena = arena.downcast_ref_unwrap();
        if size < UPLOAD_DEDICATED_THRESHOLD as u64 {
            // if the buffer is small enough, allocate through the upload buffer
            let (obj, offset) = arena
                .upload_buffer
                .write(data, self.limits.uniform_buffer_alignment)
                .unwrap();
            arena.buffers.alloc(GlBuffer {
                raw: RawBuffer {
                    obj,
                    size: size as usize,
                },
                offset,
                alias_info: None,
                should_destroy: false,
            })
        } else {
            // TODO
            unimplemented!()
        }
    }

    //----------------------------------------------------------------------------------------------
    fn create_buffer<'a>(
        &self,
        _arena: &'a dyn traits::Arena,
        _size: u64,
    ) -> &'a dyn traits::Buffer {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    fn create_shader_module<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        data: &[u8],
        stage: ShaderStageFlags,
    ) -> &'a dyn traits::ShaderModule {
        let arena: &GlArena = arena.downcast_ref_unwrap();
        // detect SPIR-V or GLSL
        // TODO big-endian is also possible!
        // FIXME clippy warning: data may be misaligned
        let module = if data.len() >= 4 && unsafe { *(data.as_ptr() as *const u32) } == SPIRV_MAGIC
        {
            assert!(data.len() % 4 == 0);
            // reinterpret as u32
            // FIXME clippy warning: data may be misaligned
            let data_u32 = unsafe {
                ::std::slice::from_raw_parts(data.as_ptr() as *const u32, data.len() / 4)
            };

            GlShaderModule {
                obj: 0,
                spirv: data_u32.to_vec().into(),
                stage,
            }
        } else {
            GlShaderModule::from_glsl(&self.gl, stage, data)
                .expect("failed to compile shader from GLSL source")
        };

        arena.shader_modules.alloc(module)
    }

    //----------------------------------------------------------------------------------------------
    fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        create_info: &GraphicsPipelineCreateInfoTypeless<'_, 'a>,
    ) -> &'a dyn traits::GraphicsPipeline {
        let arena: &GlArena = arena.downcast_ref_unwrap();
        create_graphics_pipeline_internal(&self.gl, arena, create_info)
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set_layout<'a, 'r: 'a>(
        &'r self,
        arena: &'a dyn traits::Arena,
        typeid: Option<TypeId>,
        bindings: &[DescriptorSetLayoutBinding],
    ) -> &'a dyn traits::DescriptorSetLayout {
        assert_ne!(bindings.len(), 0, "descriptor set layout has no bindings");

        if let Some(typeid) = typeid {
            self.desc_set_layout_cache
                .get_or_insert_with(typeid, || GlDescriptorSetLayout {
                    bindings: bindings.iter().map(|b| b.clone().into()).collect(),
                })
        } else {
            let arena: &GlArena = arena.downcast_ref_unwrap();
            arena.descriptor_set_layouts.alloc(GlDescriptorSetLayout {
                bindings: bindings.iter().map(|b| b.clone().into()).collect(),
            })
        }
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set<'a>(
        &self,
        arena: &'a dyn traits::Arena,
        layout: &dyn traits::DescriptorSetLayout,
        descriptors: &[Descriptor],
    ) -> &'a dyn traits::DescriptorSet {
        let arena: &GlArena = arena.downcast_ref_unwrap();
        let mut sampler_cache = self.sampler_cache.lock().unwrap();
        let descriptor_set = GlDescriptorSet::from_descriptors_and_layout(
            &self.gl,
            descriptors,
            layout.downcast_ref_unwrap(),
            &mut sampler_cache,
        );
        arena.descriptor_sets.alloc(descriptor_set)
    }

    //----------------------------------------------------------------------------------------------
    fn submit_frame<'a>(&self, frame: &[Command<'a>]) {
        //let mut rsrc = self.rsrc.lock().unwrap();
        let mut scache = self.state_cache.lock().unwrap();

        // invalidate the cache, because deletion of objects in arenas between two calls
        // to `submit_frame` may have automatically 'unbound' objects from the pipeline.
        scache.invalidate();

        // execute commands
        {
            let mut subctxt = SubmissionContext::new(&self.gl, &mut scache, &self.limits);
            for cmd in frame.iter() {
                subctxt.submit_command(cmd);
            }
        }

        let mut fnum = self.frame_num.lock().unwrap();
        let mut timeline = self.timeline.lock().unwrap();
        timeline.signal(&self.gl, *fnum);

        // wait for previous frames before starting a new one
        // if max_frames_in_flight is zero, then will wait on the previously signalled point.
        if *fnum > u64::from(self.max_frames_in_flight) {
            let timeout = !timeline.client_sync(
                &self.gl,
                *fnum - u64::from(self.max_frames_in_flight),
                FRAME_WAIT_TIMEOUT,
            );
            if timeout {
                panic!(
                    "timeout ({:?}) waiting for frame to finish",
                    FRAME_WAIT_TIMEOUT
                )
            }
        }

        *fnum += 1;
    }

    fn update_image(
        &self,
        _image: &dyn traits::Image,
        _min_extent: (u32, u32, u32),
        _max_extent: (u32, u32, u32),
        _data: &[u8],
    ) {
        unimplemented!()
    }
}
