#[macro_use]
extern crate log;

mod api;
mod buffer;
mod cmd;
mod descriptor;
mod format;
mod framebuffer;
mod image;
mod pipeline;
pub mod pipeline_file;
mod pool;
mod resource;
mod shader;
mod state;
mod sync;
mod upload;
mod util;
mod window;

pub use self::pipeline_file::PipelineDescriptionFile;
pub use self::window::create_backend_and_window;

use crate::api as gl;
use crate::api::types::*;
use crate::cmd::ExecuteCtxt;
use crate::pipeline::create_graphics_pipeline_internal;
use crate::{
    descriptor::{GlDescriptorSet, GlDescriptorSetLayout},
    framebuffer::GlFramebuffer,
    image::{upload_image_region, RawImage},
    pipeline::GlGraphicsPipeline,
    resource::{GlArena, GlBuffer, GlImage, Resources, SamplerCache},
    shader::{create_shader_from_glsl, GlShaderModule},
    state::StateCache,
    sync::Timeline,
};
use config::Config;
use autograph_render;
use autograph_render::{
    AliasScope, Command, Descriptor, DescriptorSetLayoutBinding, Dimensions, Format,
    GraphicsPipelineCreateInfo, ImageUsageFlags, MipmapsCount, RendererBackend, ShaderStageFlags,
};
use std::ffi::CStr;
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::fmt;
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
pub struct ImplementationParameters {
    pub uniform_buffer_alignment: usize,
    pub max_draw_buffers: u32,
    pub max_color_attachments: u32,
    pub max_viewports: u32,
}

impl ImplementationParameters {
    pub fn populate(gl: &gl::Gl) -> ImplementationParameters {
        let getint = |param| unsafe {
            let mut v = mem::uninitialized();
            gl.GetIntegerv(param, &mut v);
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

/// Trait implemented by objects that can act as a swapchain.
///
/// OpenGL does not have the concept of "swapchains": this is typically handled by the
/// underlying window system. This type wraps around window handles and provides an interface
/// for getting the size of the swapchain (default framebuffer) and present an image to the screen
/// (swap buffers).
pub trait SwapchainInner: Send + Sync
{
    fn size(&self) -> (u32,u32);
    fn present(&self);
}

/// Represents an OpenGL "swapchain".
pub struct GlSwapchain {
    inner: Box<dyn SwapchainInner>
}

impl fmt::Debug for GlSwapchain
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Swapchain {{..}}")
    }
}

impl autograph_render::traits::Swapchain for GlSwapchain {
    fn size(&self) -> (u32, u32) {
        self.inner.size()
    }
}

impl autograph_render::traits::GraphicsPipeline for GlGraphicsPipeline {}
impl autograph_render::traits::ShaderModule for GlShaderModule {}
impl autograph_render::traits::DescriptorSetLayout for GlDescriptorSetLayout {}
impl autograph_render::traits::DescriptorSet for GlDescriptorSet {}
impl autograph_render::traits::Buffer for GlBuffer {
    fn size(&self) -> u64 {
        self.size as u64
    }
}
impl autograph_render::traits::Image for GlImage {}
impl autograph_render::traits::Framebuffer for GlFramebuffer {}
//impl renderer::DescriptorSet for DescriptorSet {}

pub struct OpenGlBackend {
    rsrc: Mutex<Resources>,
    timeline: Mutex<Timeline>,
    frame_num: Mutex<u64>, // replace with AtomicU64 once stabilized
    state_cache: Mutex<StateCache>,
    sampler_cache: Mutex<SamplerCache>,
    limits: ImplementationParameters,
    //window: GlWindow,
    def_swapchain: GlSwapchain,
    max_frames_in_flight: u32,
    gl: gl::Gl,
}

impl OpenGlBackend {
    pub fn with_gl(cfg: &Config, gl: gl::Gl, default_swapchain: Box<dyn SwapchainInner>) -> OpenGlBackend {
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
                inner: default_swapchain
            },
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
    type Swapchain = GlSwapchain;
    type Buffer = GlBuffer;
    type Image = GlImage;
    type Framebuffer = GlFramebuffer;
    type DescriptorSet = GlDescriptorSet;
    type DescriptorSetLayout = GlDescriptorSetLayout;
    type ShaderModule = GlShaderModule;
    type GraphicsPipeline = GlGraphicsPipeline;
    type Arena = GlArena;

    fn create_arena(&self) -> Self::Arena {
        self.rsrc.lock().unwrap().create_arena(&self.gl)
    }

    fn drop_arena(&self, arena: Self::Arena) {
        self.rsrc.lock().unwrap().drop_arena(&self.gl, arena)
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
            .alloc_aliased_image(&self.gl, arena, scope, fmt, dims, mipcount, samples, usage)
    }

    //----------------------------------------------------------------------------------------------

    /// Creates a framebuffer. See trait documentation for explanation of unsafety.
    fn create_framebuffer<'a>(
        &self,
        arena: &'a Self::Arena,
        color_att: &[autograph_render::Image<'a, Self>],
        depth_stencil_att: Option<autograph_render::Image<'a, Self>>,
    ) -> &'a Self::Framebuffer {
        arena
            .framebuffers
            .alloc(GlFramebuffer::new(&self.gl, color_att, depth_stencil_att).unwrap())
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
            arena.buffers.alloc(GlBuffer {
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

            GlShaderModule {
                obj: 0,
                spirv: data_u32.to_vec().into(),
                stage,
            }
        } else {
            let obj = create_shader_from_glsl(&self.gl, stage, data)
                .expect("failed to compile shader from GLSL source");
            GlShaderModule {
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
    ) -> &'a GlGraphicsPipeline {
        create_graphics_pipeline_internal(&self.gl, arena, create_info)
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set_layout<'a>(
        &self,
        arena: &'a Self::Arena,
        bindings: &[DescriptorSetLayoutBinding],
    ) -> &'a GlDescriptorSetLayout {
        assert_ne!(bindings.len(), 0, "descriptor set layout has no bindings");
        arena.descriptor_set_layouts.alloc(GlDescriptorSetLayout {
            bindings: bindings.iter().map(|b| b.clone().into()).collect(),
        })
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set<'a>(
        &self,
        arena: &'a Self::Arena,
        layout: &GlDescriptorSetLayout,
        descriptors: &[Descriptor<Self>],
    ) -> &'a GlDescriptorSet {
        let mut sampler_cache = self.sampler_cache.lock().unwrap();
        let descriptor_set =
            GlDescriptorSet::from_descriptors_and_layout(&self.gl, descriptors, layout, &mut sampler_cache);
        arena.descriptor_sets.alloc(descriptor_set)
    }

    //----------------------------------------------------------------------------------------------
    fn submit_frame<'a>(&self, frame: &[Command<'a, Self>]) {
        let mut rsrc = self.rsrc.lock().unwrap();
        let mut scache = self.state_cache.lock().unwrap();

        // execute commands
        {
            let mut ectxt = ExecuteCtxt::new(&self.gl, &mut rsrc, &mut scache, &self.limits);
            for cmd in frame.iter() {
                ectxt.execute_command(cmd);
            }
        }

        let mut fnum = self.frame_num.lock().unwrap();
        let mut timeline = self.timeline.lock().unwrap();
        timeline.signal(&self.gl, *fnum);

        // wait for previous frames before starting a new one
        // if max_frames_in_flight is zero, then will wait on the previously signalled point.
        if *fnum > u64::from(self.max_frames_in_flight) {
            let timeout = !timeline.client_sync(&self.gl,
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

    fn update_image(&self, image: &GlImage,
                    min_extent: (u32, u32, u32),
                    max_extent: (u32, u32, u32),
                    data: &[u8])
    {
        unimplemented!()
    }
}

//--------------------------------------------------------------------------------------------------
pub type Backend = OpenGlBackend;
pub type Buffer<'a, T> = autograph_render::Buffer<'a, OpenGlBackend, T>;
pub type BufferTypeless<'a> = autograph_render::BufferTypeless<'a, OpenGlBackend>;
pub type Image<'a> = autograph_render::Image<'a, OpenGlBackend>;
pub type Framebuffer<'a> = autograph_render::Framebuffer<'a, OpenGlBackend>;
pub type DescriptorSet<'a> = autograph_render::DescriptorSet<'a, OpenGlBackend>;
pub type DescriptorSetLayout<'a> = autograph_render::DescriptorSetLayout<'a, OpenGlBackend>;
pub type GraphicsPipeline<'a> = autograph_render::GraphicsPipeline<'a, OpenGlBackend>;
pub type Arena<'a> = autograph_render::Arena<'a, OpenGlBackend>;
