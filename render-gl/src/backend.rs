use crate::{
    aliaspool::AliasPool,
    api as gl,
    api::{types::*, Gl},
    buffer::{create_buffer, GlBuffer, MappedBuffer, RawBuffer, UploadBuffer},
    command::{StateCache, SubmissionContext},
    framebuffer::GlFramebuffer,
    image::{upload_image_region, GlImage, ImageAliasKey, ImageDescription, RawImage},
    pipeline::{
        create_graphics_pipeline_internal, GlArgumentBlock, GlGraphicsPipeline, GlShaderModule,
        GlSignature,
    },
    sampler::SamplerCache,
    swapchain::GlSwapchain,
    sync::{GpuSyncObject, Timeline},
    util::DroplessArena,
    AliasInfo, ImplementationParameters,
};
use autograph_render::{
    command::Command,
    descriptor::Descriptor,
    format::Format,
    image::{DepthStencilView, Dimensions, ImageUsageFlags, MipmapsOption, RenderTargetView},
    pipeline::{
        BareArgumentBlock, GraphicsPipelineCreateInfo, Scissor, ShaderStageFlags,
        SignatureDescription, Viewport,
    },
    vertex::{IndexBufferView, VertexBufferView},
    AliasScope, Backend, Instance,
};
use config::Config;
use glutin::{GlContext, GlWindow};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ffi::CStr,
    mem,
    os::raw::c_char,
    ptr, slice, str,
    sync::Arc,
    time::Duration,
};
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
        desc: &ImageDescription,
    ) -> &'a GlImage {
        let (key, raw) = self
            .image_pool
            .alloc(scope, *desc, |d| RawImage::new(gl, d));

        arena.images.alloc(GlImage {
            alias_info: AliasInfo { key, scope }.into(),
            raw: raw.clone(),
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
    limits: ImplementationParameters,
    window: Option<Arc<GlWindow>>,
    def_swapchain: Option<GlSwapchain>,
    cfg: InstanceConfig,
    gl: gl::Gl,
}

#[derive(Copy, Clone, Debug)]
pub struct InstanceConfig {
    pub upload_buffer_size: usize,
    pub max_frames_in_flight: u32,
    pub vsync: bool,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        InstanceConfig {
            upload_buffer_size: 4 * 1024 * 1024,
            max_frames_in_flight: 1,
            vsync: false,
        }
    }
}

impl OpenGlInstance {
    /// Returns the associated [glutin::GlWindow] if there is one.
    pub fn window(&self) -> Option<&Arc<GlWindow>> {
        self.window.as_ref()
    }

    pub fn from_gl_window(cfg: &InstanceConfig, window: Arc<GlWindow>) -> OpenGlInstance {
        let gl = unsafe {
            // Make current the OpenGL context associated to the window
            // and load function pointers
            window.make_current().unwrap();

            crate::api::Gl::load_with(|symbol| {
                let ptr = window.get_proc_address(symbol) as *const _;
                ptr
            })
        };

        unsafe {
            // Enable debug output
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

            info!(
                "OpenGL version {}.{} (vendor: {:?}, renderer: {:?})",
                major_version, minor_version, vendor, renderer
            );
        }

        let timeline = Timeline::new(0);

        let limits = ImplementationParameters::populate(&gl);
        let state_cache = StateCache::new(&limits);

        OpenGlInstance {
            rsrc: RefCell::new(Resources::new(cfg.upload_buffer_size)),
            timeline: RefCell::new(timeline),
            frame_num: Cell::new(1),
            window: Some(window.clone()),
            def_swapchain: Some(GlSwapchain {
                window: window.clone(),
            }),
            gl,
            cfg: *cfg,
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
        self.def_swapchain.as_ref()
    }

    //----------------------------------------------------------------------------------------------
    unsafe fn create_image<'a>(
        &self,
        arena: &'a GlArena,
        scope: AliasScope,
        format: Format,
        dimensions: Dimensions,
        mipmaps: MipmapsOption,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: Option<&[u8]>,
    ) -> &'a GlImage {
        let d = ImageDescription::new(format, dimensions, mipmaps, samples, usage);

        if scope != AliasScope::no_alias() {
            // cannot specify initial data for aliasable image
            assert!(initial_data.is_none());
            self.rsrc
                .borrow_mut()
                .alloc_aliased_image(&self.gl, arena, scope, &d)
        } else {
            // not aliasable, dedicated allocation
            let raw = RawImage::new(&self.gl, &d);

            if let Some(data) = initial_data {
                unsafe {
                    upload_image_region(
                        &self.gl,
                        raw.target,
                        raw.obj,
                        format,
                        0,
                        (0, 0, 0),
                        dimensions.width_height_depth(),
                        data,
                    );
                }
            }

            arena.images.alloc(GlImage {
                should_destroy: true,
                raw,
                alias_info: None,
            })
        }
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
            // otherwise, allocate a dedicated buffer
            arena.buffers.alloc(GlBuffer {
                raw: RawBuffer {
                    obj: create_buffer(&self.gl, size as usize, 0, Some(data)),
                    size: size as usize,
                },
                offset: 0,
                should_destroy: true,
                alias_info: None,
            })
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
    unsafe fn create_argument_block<'a>(
        &self,
        arena: &'a GlArena,
        signature: &'a GlSignature,
        inherited: impl IntoIterator<Item = BareArgumentBlock<'a, OpenGlBackend>>,
        descriptors: impl IntoIterator<Item = Descriptor<'a, OpenGlBackend>>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferView<'a, OpenGlBackend>>,
        index_buffer: Option<IndexBufferView<'a, OpenGlBackend>>,
        render_targets: impl IntoIterator<Item = RenderTargetView<'a, OpenGlBackend>>,
        depth_stencil_render_target: Option<DepthStencilView<'a, OpenGlBackend>>,
        viewports: impl IntoIterator<Item = Viewport>,
        scissors: impl IntoIterator<Item = Scissor>,
    ) -> &'a GlArgumentBlock {
        let mut sampler_cache = self.sampler_cache.borrow_mut();
        GlArgumentBlock::new(
            arena,
            &self.gl,
            &mut sampler_cache,
            signature,
            inherited,
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

        //self.gl.ClipControl(gl::UPPER_LEFT, gl::NEGATIVE_ONE_TO_ONE);
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
        if fnum > u64::from(self.cfg.max_frames_in_flight) {
            let timeout = !timeline.client_sync(
                &self.gl,
                fnum - u64::from(self.cfg.max_frames_in_flight),
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
