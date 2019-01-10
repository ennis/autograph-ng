use crate::api as gl;
use crate::api::types::*;
use crate::cmd::ExecuteCtxt;
use crate::pipeline::create_graphics_pipeline_internal;
use crate::{
    descriptor::{DescriptorSet, DescriptorSetLayout},
    framebuffer::Framebuffer,
    image::{upload_image_region, RawImage},
    pipeline::GraphicsPipeline,
    resource::{Arena, Buffer, Image, Resources, SamplerCache},
    shader::{create_shader_from_glsl, ShaderModule},
    state::StateCache,
    sync::Timeline,
};
use config::Config;
use gfx2;
use gfx2::{
    AliasScope, Command, Descriptor, DescriptorSetLayoutBinding, Dimensions, Format,
    GraphicsPipelineCreateInfo, ImageUsageFlags, MipmapsCount, RendererBackend, ShaderStageFlags,
};
use glutin::{GlContext, GlWindow};
use std::ffi::CStr;
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::str;
use std::sync::Mutex;
use std::time::Duration;

//pub use self::pipeline::{create_graphics_pipeline_internal, GlGraphicsPipeline};

//pub use self::{descriptor::DescriptorSet, descriptor::DescriptorSetLayout, };

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
pub struct ImplementationParameters {
    pub uniform_buffer_alignment: usize,
    pub max_draw_buffers: u32,
    pub max_color_attachments: u32,
    pub max_viewports: u32,
}

impl ImplementationParameters {
    pub fn populate() -> ImplementationParameters {
        let getint = |param| unsafe {
            let mut v = mem::uninitialized();
            gl::GetIntegerv(param, &mut v);
            v
        };

        ImplementationParameters {
            uniform_buffer_alignment: getint(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT) as usize,
            max_draw_buffers: getint(gl::MAX_DRAW_BUFFERS) as u32,
            max_color_attachments: getint(gl::MAX_COLOR_ATTACHMENTS) as u32,
            max_viewports: getint(gl::MAX_VIEWPORTS) as u32,
        }
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct Swapchain {
    size: Mutex<(u32, u32)>,
}

impl gfx2::traits::Swapchain for Swapchain {
    fn size(&self) -> (u32, u32) {
        *self.size.lock().unwrap()
    }
}

impl gfx2::traits::GraphicsPipeline for GraphicsPipeline {}
impl gfx2::traits::ShaderModule for ShaderModule {}
impl gfx2::traits::DescriptorSetLayout for DescriptorSetLayout {}
impl gfx2::traits::DescriptorSet for DescriptorSet {}
impl gfx2::traits::Buffer for Buffer {
    fn size(&self) -> u64 {
        self.size as u64
    }
}
impl gfx2::traits::Image for Image {}
impl gfx2::traits::Framebuffer for Framebuffer {}
//impl renderer::DescriptorSet for DescriptorSet {}

pub struct OpenGlBackend {
    rsrc: Mutex<Resources>,
    timeline: Mutex<Timeline>,
    frame_num: Mutex<u64>, // replace with AtomicU64 once stabilized
    state_cache: Mutex<StateCache>,
    sampler_cache: Mutex<SamplerCache>,
    limits: ImplementationParameters,
    window: GlWindow,
    def_swapchain: Swapchain,
    max_frames_in_flight: u32,
}

impl OpenGlBackend {
    pub fn with_gl_window(cfg: &Config, w: GlWindow) -> OpenGlBackend {
        // Make current the OpenGL context associated to the window
        // and load function pointers
        unsafe { w.make_current() }.unwrap();
        gl::load_with(|symbol| {
            let ptr = w.get_proc_address(symbol) as *const _;
            //debug!("getProcAddress {} -> {:?}", symbol, ptr);
            ptr
        });

        unsafe {
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(debug_callback as GLDEBUGPROC, ptr::null());
            gl::DebugMessageControl(
                gl::DONT_CARE,
                gl::DONT_CARE,
                gl::DONT_CARE,
                0,
                ptr::null(),
                1,
            );

            let mut major_version = mem::uninitialized();
            let mut minor_version = mem::uninitialized();
            gl::GetIntegerv(gl::MAJOR_VERSION, &mut major_version);
            gl::GetIntegerv(gl::MINOR_VERSION, &mut minor_version);

            let vendor = CStr::from_ptr(gl::GetString(gl::VENDOR) as *const c_char);
            let renderer = CStr::from_ptr(gl::GetString(gl::RENDERER) as *const c_char);

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

        let limits = ImplementationParameters::populate();
        let state_cache = StateCache::new(&limits);

        OpenGlBackend {
            rsrc: Mutex::new(Resources::new(upload_buffer_size as usize)),
            timeline: Mutex::new(timeline),
            frame_num: Mutex::new(1),
            def_swapchain: Swapchain {
                size: Mutex::new(w.get_inner_size().unwrap().into()),
            },
            window: w,
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
    type Swapchain = Swapchain;
    type Buffer = Buffer;
    type Image = Image;
    type Framebuffer = Framebuffer;
    type DescriptorSet = DescriptorSet;
    type DescriptorSetLayout = DescriptorSetLayout;
    type ShaderModule = ShaderModule;
    type GraphicsPipeline = GraphicsPipeline;
    type Arena = Arena;

    fn create_arena(&self) -> Self::Arena {
        self.rsrc.lock().unwrap().create_arena()
    }

    fn drop_arena(&self, arena: Self::Arena) {
        self.rsrc.lock().unwrap().drop_arena(arena)
    }

    //----------------------------------------------------------------------------------------------
    fn create_swapchain<'a>(&self, _arena: &'a Self::Arena) -> &'a Self::Swapchain {
        unimplemented!()
    }

    fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx Self::Swapchain> {
        Some(&self.def_swapchain)
    }

    //----------------------------------------------------------------------------------------------
    fn create_immutable_image<'a>(
        &self,
        arena: &'a Self::Arena,
        fmt: Format,
        dims: Dimensions,
        mips: MipmapsCount,
        samples: u32,
        _usage: ImageUsageFlags,
        data: &[u8],
    ) -> &'a Self::Image {
        // initial data specified, allocate a texture
        let raw = RawImage::new_texture(fmt, &dims, mips, samples);

        unsafe {
            upload_image_region(
                raw.target,
                raw.obj,
                fmt,
                0,
                (0, 0, 0),
                dims.width_height_depth(),
                data,
            );
        }

        arena.images.alloc(Image {
            should_destroy: true,
            obj: raw.obj,
            target: raw.target,
            alias_info: None,
        })
    }

    //----------------------------------------------------------------------------------------------
    fn create_image<'a>(
        &self,
        arena: &'a Self::Arena,
        scope: AliasScope,
        fmt: Format,
        dims: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a Self::Image {
        self.rsrc
            .lock()
            .unwrap()
            .alloc_aliased_image(arena, scope, fmt, dims, mipcount, samples, usage)
    }

    //----------------------------------------------------------------------------------------------

    /// Creates a framebuffer. See trait documentation for explanation of unsafety.
    fn create_framebuffer<'a>(
        &self,
        arena: &'a Self::Arena,
        color_att: &[gfx2::Image<'a, Self>],
        depth_stencil_att: Option<gfx2::Image<'a, Self>>,
    ) -> &'a Self::Framebuffer {
        arena
            .framebuffers
            .alloc(Framebuffer::new(color_att, depth_stencil_att).unwrap())
    }

    //----------------------------------------------------------------------------------------------
    fn create_immutable_buffer<'a>(
        &self,
        arena: &'a Self::Arena,
        size: u64,
        data: &[u8],
    ) -> &'a Self::Buffer {
        if size < UPLOAD_DEDICATED_THRESHOLD as u64 {
            // if the buffer is small enough, allocate through the upload buffer
            let (obj, offset) = arena
                .upload_buffer
                .write(data, self.limits.uniform_buffer_alignment)
                .unwrap();
            arena.buffers.alloc(Buffer {
                obj,
                offset,
                size: size as usize,
                alias_info: None,
                should_destroy: false,
            })
        } else {
            // TODO
            unimplemented!()
        }
    }

    //----------------------------------------------------------------------------------------------
    fn create_buffer<'a>(&self, _arena: &'a Self::Arena, _size: u64) -> &'a Self::Buffer {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    fn create_shader_module<'a>(
        &self,
        arena: &'a Self::Arena,
        data: &[u8],
        stage: ShaderStageFlags,
    ) -> &'a Self::ShaderModule {
        // detect SPIR-V or GLSL
        // TODO big-endian is also possible!
        // FIXME clippy warning: data may be misaligned
        let module = if data.len() >= 4 && unsafe { *(data.as_ptr() as *const u32) } == SPIRV_MAGIC
        {
            assert!(data.len() % 4 == 0);
            // reinterpret as u32
            let data_u32 = unsafe {
                // FIXME clippy warning: data may be misaligned
                ::std::slice::from_raw_parts(data.as_ptr() as *const u32, data.len() / 4)
            };

            ShaderModule {
                obj: 0,
                spirv: data_u32.to_vec().into(),
                stage,
            }
        } else {
            let obj = create_shader_from_glsl(stage, data)
                .expect("failed to compile shader from GLSL source");
            ShaderModule {
                obj,
                spirv: None,
                stage,
            }
        };

        arena.shader_modules.alloc(module)
    }

    //----------------------------------------------------------------------------------------------
    fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a Self::Arena,
        create_info: &GraphicsPipelineCreateInfo<'_, 'a, Self>,
    ) -> &'a GraphicsPipeline {
        create_graphics_pipeline_internal(arena, create_info)
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set_layout<'a>(
        &self,
        arena: &'a Self::Arena,
        bindings: &[DescriptorSetLayoutBinding],
    ) -> &'a DescriptorSetLayout {
        assert_ne!(bindings.len(), 0, "descriptor set layout has no bindings");
        arena.descriptor_set_layouts.alloc(DescriptorSetLayout {
            bindings: bindings.iter().map(|b| b.clone().into()).collect(),
        })
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set<'a>(
        &self,
        arena: &'a Self::Arena,
        layout: &DescriptorSetLayout,
        descriptors: &[Descriptor<Self>],
    ) -> &'a DescriptorSet {
        let mut sampler_cache = self.sampler_cache.lock().unwrap();
        let descriptor_set =
            DescriptorSet::from_descriptors_and_layout(descriptors, layout, &mut sampler_cache);
        arena.descriptor_sets.alloc(descriptor_set)
    }

    //----------------------------------------------------------------------------------------------
    fn submit_frame<'a>(&self, frame: &[Command<'a, Self>]) {
        let mut rsrc = self.rsrc.lock().unwrap();
        let mut scache = self.state_cache.lock().unwrap();

        // execute commands
        {
            let mut ectxt = ExecuteCtxt::new(&mut rsrc, &mut scache, &self.window, &self.limits);
            for cmd in frame.iter() {
                ectxt.execute_command(cmd);
            }
        }

        let mut fnum = self.frame_num.lock().unwrap();
        let mut timeline = self.timeline.lock().unwrap();
        timeline.signal(*fnum);

        // wait for previous frames before starting a new one
        // if max_frames_in_flight is zero, then will wait on the previously signalled point.
        if *fnum > u64::from(self.max_frames_in_flight) {
            let timeout = !timeline.client_sync(
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
        // update default framebuffer size
        *self.def_swapchain.size.lock().unwrap() = self.window.get_inner_size().unwrap().into();
    }

    fn update_image(&self, image: &Image,
                    min_extent: (u32, u32, u32),
                    max_extent: (u32, u32, u32),
                    data: &[u8])
    {
        unimplemented!()
    }
}
