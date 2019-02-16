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
use crate::framebuffer::GlFramebuffer;
use crate::image::upload_image_region;
use crate::image::GlImage;
use crate::image::ImageAliasKey;
use crate::image::ImageDescription;
use crate::image::RawImage;
use crate::pipeline::create_graphics_pipeline_internal;
use crate::pipeline::GlArgumentBlock;
use crate::pipeline::GlGraphicsPipeline;
use crate::pipeline::GlShaderModule;
use crate::pipeline::GlSignature;
use crate::sampler::SamplerCache;
use crate::swapchain::GlSwapchain;
use crate::swapchain::SwapchainInner;
use crate::sync::GpuSyncObject;
use crate::sync::Timeline;
use crate::util::DroplessArena;
use crate::AliasInfo;
use crate::ImplementationParameters;
use autograph_render::command::Command;
use autograph_render::descriptor::Descriptor;
use autograph_render::format::Format;
use autograph_render::framebuffer::RenderTargetDescriptor;
use autograph_render::image::Dimensions;
use autograph_render::image::ImageUsageFlags;
use autograph_render::image::MipmapsCount;
use autograph_render::pipeline::BareArgumentBlock;
use autograph_render::pipeline::GraphicsPipelineCreateInfo;
use autograph_render::pipeline::ScissorRect;
use autograph_render::pipeline::ShaderStageFlags;
use autograph_render::pipeline::SignatureDescription;
use autograph_render::pipeline::Viewport;
use autograph_render::vertex::IndexBufferDescriptor;
use autograph_render::vertex::VertexBufferDescriptor;
use autograph_render::AliasScope;
use autograph_render::Backend;
use autograph_render::Instance;
use config::Config;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::CStr;
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::str;
use std::time::Duration;
use typed_arena::Arena;

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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct OpenGlBackend;

impl Backend for OpenGlBackend {
    type Instance = OpenGlInstance;
    type Arena = GlArena;
    type Swapchain = GlSwapchain;
    type Image = GlImage;
    type Buffer = GlBuffer;
    type ShaderModule = GlShaderModule;
    type GraphicsPipeline = GlGraphicsPipeline;
    type Signature = GlSignature;
    type ArgumentBlock = GlArgumentBlock;
    type HostReference = ();
}

//--------------------------------------------------------------------------------------------------
pub struct GlArena {
    pub(crate) _swapchains: Arena<GlSwapchain>,
    pub(crate) buffers: Arena<GlBuffer>,
    pub(crate) images: Arena<GlImage>,
    pub(crate) shader_modules: Arena<GlShaderModule>,
    pub(crate) signatures: Arena<GlSignature>,
    pub(crate) graphics_pipelines: Arena<GlGraphicsPipeline>,
    pub(crate) framebuffers: Arena<GlFramebuffer>,
    pub(crate) upload_buffer: UploadBuffer,
    pub(crate) other: DroplessArena,
}

impl GlArena {
    pub(crate) fn new(upload_buffer: UploadBuffer) -> GlArena {
        GlArena {
            _swapchains: Arena::new(),
            buffers: Arena::new(),
            images: Arena::new(),
            shader_modules: Arena::new(),
            signatures: Arena::new(),
            graphics_pipelines: Arena::new(),
            framebuffers: Arena::new(),
            upload_buffer,
            other: DroplessArena::new(),
        }
    }
}

//--------------------------------------------------------------------------------------------------
pub(crate) type ImagePool = AliasPool<ImageDescription, ImageAliasKey, RawImage>;
//pub(crate) type BufferPool = AliasPool<BufferDescription, BufferAliasKey, RawBuffer>;

///
struct Resources {
    image_pool: ImagePool,
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
pub struct OpenGlInstance {
    rsrc: RefCell<Resources>,
    timeline: RefCell<Timeline>,
    frame_num: Cell<u64>, // replace with AtomicU64 once stabilized
    state_cache: RefCell<StateCache>,
    sampler_cache: RefCell<SamplerCache>,
    //pipeline_signature_cache: PipelineSignatureCache,
    //desc_set_layout_cache: SyncArenaHashMap<TypeId, GlDescriptorSetLayout>,
    limits: ImplementationParameters,
    //window: GlWindow,
    def_swapchain: GlSwapchain,
    max_frames_in_flight: u32,
    gl: gl::Gl,
}

impl OpenGlInstance {
    pub fn with_gl(
        cfg: &Config,
        gl: gl::Gl,
        default_swapchain: Box<dyn SwapchainInner>,
    ) -> OpenGlInstance {
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

        OpenGlInstance {
            rsrc: RefCell::new(Resources::new(upload_buffer_size as usize)),
            timeline: RefCell::new(timeline),
            frame_num: Cell::new(1),
            def_swapchain: GlSwapchain {
                inner: default_swapchain,
            },
            gl,
            max_frames_in_flight,
            limits,
            state_cache: RefCell::new(state_cache),
            sampler_cache: RefCell::new(SamplerCache::new()),
        }
    }

    /// Creates a new OpenGlBackend from the current OpenGL context.
    ///
    /// Panics if no context is currently bound, or if the current context does not
    /// satisfy the minimum requirements of the backend implementation.
    ///
    pub fn from_current_context() -> OpenGlInstance {
        // get version, check 4.6, or DSA + SPIR-V
        unimplemented!()
    }
}

// TODO move this into a function in the spirv module
const SPIRV_MAGIC: u32 = 0x0723_0203;
const UPLOAD_DEDICATED_THRESHOLD: usize = 65536;
const FRAME_WAIT_TIMEOUT: Duration = Duration::from_millis(500);

impl Instance<OpenGlBackend> for OpenGlInstance {
    unsafe fn create_arena(&self) -> Box<GlArena> {
        self.rsrc.borrow_mut().create_arena(&self.gl)
    }

    unsafe fn drop_arena(&self, arena: Box<GlArena>) {
        self.rsrc.borrow_mut().drop_arena(&self.gl, arena)
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn create_swapchain<'a>(&self, _arena: &'a GlArena) -> &'a GlSwapchain {
        unimplemented!()
    }

    unsafe fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx GlSwapchain> {
        Some((&self.def_swapchain).into())
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn create_immutable_image<'a>(
        &self,
        arena: &'a GlArena,
        fmt: Format,
        dims: Dimensions,
        mips: MipmapsCount,
        samples: u32,
        _usage: ImageUsageFlags,
        data: &[u8],
    ) -> &'a GlImage {
        // initial data specified, allocate a texture
        let raw = RawImage::new_texture(&self.gl, fmt, &dims, mips, samples);

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

        arena.images.alloc(GlImage {
            should_destroy: true,
            raw,
            alias_info: None,
        })
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn create_image<'a>(
        &self,
        arena: &'a GlArena,
        scope: AliasScope,
        fmt: Format,
        dims: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a GlImage {
        self.rsrc
            .borrow_mut()
            .alloc_aliased_image(&self.gl, arena, scope, fmt, dims, mipcount, samples, usage)
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn create_immutable_buffer<'a>(
        &self,
        arena: &'a GlArena,
        size: u64,
        data: &[u8],
    ) -> &'a GlBuffer {
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
    unsafe fn create_buffer<'a>(&self, _arena: &'a GlArena, _size: u64) -> &'a GlBuffer {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn create_shader_module<'a>(
        &self,
        arena: &'a GlArena,
        data: &[u8],
        stage: ShaderStageFlags,
    ) -> &'a GlShaderModule {
        // detect SPIR-V or GLSL
        // TODO big-endian is also possible!
        // FIXME clippy warning: data may be misaligned
        let module = if data.len() >= 4 && *(data.as_ptr() as *const u32) == SPIRV_MAGIC {
            assert!(data.len() % 4 == 0);
            // reinterpret as u32
            // FIXME clippy warning: data may be misaligned
            let data_u32 =
                ::std::slice::from_raw_parts(data.as_ptr() as *const u32, data.len() / 4);

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
    unsafe fn create_graphics_pipeline<'a, 'b>(
        &self,
        arena: &'a GlArena,
        root_signature: &'a GlSignature,
        root_signature_description: &SignatureDescription,
        create_info: &GraphicsPipelineCreateInfo<'a, 'b, OpenGlBackend>,
    ) -> &'a GlGraphicsPipeline {
        create_graphics_pipeline_internal(
            &self.gl,
            arena,
            root_signature,
            root_signature_description,
            create_info,
        )
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn create_argument_block<'a, 'b>(
        &self,
        arena: &'a GlArena,
        signature: &'a GlSignature,
        arguments: impl IntoIterator<Item = BareArgumentBlock<'a, OpenGlBackend>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, OpenGlBackend>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferDescriptor<'a, 'b, OpenGlBackend>>,
        index_buffer: Option<IndexBufferDescriptor<'a, OpenGlBackend>>,
        render_targets: impl IntoIterator<Item = RenderTargetDescriptor<'a, OpenGlBackend>>,
        depth_stencil_render_target: Option<RenderTargetDescriptor<'a, OpenGlBackend>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = ScissorRect>,
    ) -> &'a GlArgumentBlock {
        let mut sampler_cache = self.sampler_cache.borrow_mut();
        GlArgumentBlock::new(
            arena,
            &self.gl,
            &mut sampler_cache,
            signature,
            arguments,
            descriptors,
            vertex_buffers,
            index_buffer,
            render_targets,
            depth_stencil_render_target,
            viewports,
            scissors,
        )
    }

    unsafe fn create_signature<'a>(
        &'a self,
        arena: &'a GlArena,
        inherited: &[&'a GlSignature],
        description: &SignatureDescription,
    ) -> &'a GlSignature {
        let sig = GlSignature::new(arena, inherited, description);

        sig
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn create_host_reference<'a>(&self, _arena: &'a GlArena, _data: &'a [u8]) -> &'a () {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn submit_frame<'a>(&self, frame: &[Command<'a, OpenGlBackend>]) {
        let mut scache = self.state_cache.borrow_mut();

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

        let fnum = self.frame_num.get();
        let mut timeline = self.timeline.borrow_mut();
        timeline.signal(&self.gl, fnum);

        // wait for previous frames before starting a new one
        // if max_frames_in_flight is zero, then will wait on the previously signalled point.
        if fnum > u64::from(self.max_frames_in_flight) {
            let timeout = !timeline.client_sync(
                &self.gl,
                fnum - u64::from(self.max_frames_in_flight),
                FRAME_WAIT_TIMEOUT,
            );
            if timeout {
                panic!(
                    "timeout ({:?}) waiting for frame to finish",
                    FRAME_WAIT_TIMEOUT
                )
            }
        }

        self.frame_num.set(fnum + 1);
    }

    unsafe fn update_image(
        &self,
        _image: &GlImage,
        _min_extent: (u32, u32, u32),
        _max_extent: (u32, u32, u32),
        _data: &[u8],
    ) {
        unimplemented!()
    }
}
