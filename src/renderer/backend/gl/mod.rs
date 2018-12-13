use glutin::{GlContext, GlWindow};
use std::cell::Cell;
use std::collections::VecDeque;
use std::ffi::CStr;
use std::mem;
use std::ops::DerefMut;
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::str;
use std::sync::Mutex;

// OpenGL API
mod api;
// Buffer objects
mod buffer;
// Resource pool
mod pool;
// OpenGL formats
mod format;
// Image objects
mod image;
// Pipeline objects (programs + VAO + state)
mod pipeline;
// shader objects
mod shader;
// pipeline stage management
mod state;
// synchronization primitives
mod sync;
// upload buffers
mod command;
mod descriptor;
mod framebuffer;
mod resource;
mod upload;
mod window;

// pipeline files
pub mod pipeline_file;

use config::Config;
//use sid_vec::{FromIndex, Id, IdVec};
use ordered_float::NotNan;
use slotmap::SlotMap;
use smallvec::SmallVec;

use self::api as gl;
use self::api::types::*;
use self::command::ExecuteContext;
use crate::renderer;
use crate::renderer::command_buffer::*;
use crate::renderer::util;
use crate::renderer::{
    AliasScope, AttachmentDescription, Descriptor, DescriptorSetLayoutBinding, DescriptorType,
    Dimensions, Format, GraphicsPipelineCreateInfo, ImageUsageFlags, MipmapsCount, RendererBackend,
    ShaderStageFlags,
};

use self::{
    buffer::RawBuffer,
    descriptor::{DescriptorSet, DescriptorSetLayout},
    framebuffer::Framebuffer,
    image::{upload_image_region, RawImage},
    resource::{Arena, Buffer, Image, Resources, SamplerCache},
    shader::{create_shader_from_glsl, ShaderModule},
    state::StateCache,
    sync::{Timeline, Timeout},
};

pub use self::pipeline::{
    create_graphics_pipeline_internal, GraphicsPipeline, GraphicsPipelineCreateInfoAdditional,
};
pub use self::pipeline_file::PipelineDescriptionFile;
pub use self::window::create_backend_and_window;

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

impl renderer::SwapchainBackend for Swapchain {
    fn size(&self) -> (u32, u32) {
        *self.size.lock().unwrap()
    }
}

pub struct OpenGlBackend {
    resources: Mutex<Resources>,
    timeline: Mutex<Timeline>,
    frame_number: Mutex<u64>, // replace with AtomicU64 once stabilized
    state_cache: Mutex<StateCache>,
    sampler_cache: Mutex<SamplerCache>,
    impl_params: ImplementationParameters,
    window: GlWindow,
    default_swapchain: Swapchain,
    max_frames_in_flight: u32,
}

impl OpenGlBackend {
    pub fn with_gl_window(cfg: &Config, window: GlWindow) -> OpenGlBackend {
        // Make current the OpenGL context associated to the window
        // and load function pointers
        unsafe { window.make_current() }.unwrap();
        gl::load_with(|symbol| {
            let ptr = window.get_proc_address(symbol) as *const _;
            debug!("getProcAddress {} -> {:?}", symbol, ptr);
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

        let upload_buffer_size = cfg.get::<u64>("gfx.default_upload_buffer_size").unwrap();
        assert!(upload_buffer_size <= usize::max_value() as u64);
        let max_frames_in_flight = cfg.get::<u32>("gfx.max_frames_in_flight").unwrap();

        let mut timeline = Timeline::new(0);

        let impl_params = ImplementationParameters::populate();
        let state_cache = StateCache::new(&impl_params);

        OpenGlBackend {
            resources: Mutex::new(Resources::new(upload_buffer_size as usize)),
            timeline: Mutex::new(timeline),
            frame_number: Mutex::new(1),
            default_swapchain: Swapchain {
                size: Mutex::new(window.get_inner_size().unwrap().into()),
            },
            window,
            max_frames_in_flight,
            impl_params,
            state_cache: Mutex::new(state_cache),
            sampler_cache: Mutex::new(SamplerCache::new()),
        }
    }
}

// TODO move this into a function in the spirv module
const SPIRV_MAGIC: u32 = 0x0723_0203;
const UPLOAD_DEDICATED_THRESHOLD: usize = 65536;

impl renderer::GraphicsPipelineBackend for GraphicsPipeline {}
impl renderer::ShaderModuleBackend for ShaderModule {}
impl renderer::DescriptorSetLayoutBackend for DescriptorSetLayout {}
impl renderer::BufferBackend for Buffer {
    fn size(&self) -> u64 {
        self.size as u64
    }
}
impl renderer::ImageBackend for Image {}
impl renderer::FramebufferBackend for Framebuffer {}
//impl renderer::DescriptorSet for DescriptorSet {}

impl RendererBackend for OpenGlBackend {
    type Swapchain = Swapchain;
    type Buffer = Buffer;
    type Image = Image;
    type Framebuffer = Framebuffer;
    type DescriptorSet = DescriptorSet;
    type DescriptorSetLayout = DescriptorSetLayout;
    type ShaderModule = ShaderModule;
    type GraphicsPipeline = GraphicsPipeline;
    type GraphicsPipelineCreateInfoAdditional = GraphicsPipelineCreateInfoAdditional;
    type Arena = Arena;
    //type AttachmentLayoutHandle = AttachmentLayoutHandle;

    fn create_arena(&self) -> Self::Arena {
        self.resources.lock().unwrap().create_arena()
    }

    fn drop_arena(&self, arena: Self::Arena) {
        self.resources.lock().unwrap().drop_arena(arena)
    }

    //----------------------------------------------------------------------------------------------
    fn create_swapchain<'a>(&self, arena: &'a Self::Arena) -> &'a Self::Swapchain {
        unimplemented!()
    }

    fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx Self::Swapchain> {
        Some(&self.default_swapchain)
    }

    //----------------------------------------------------------------------------------------------
    fn create_immutable_image<'a>(
        &self,
        arena: &'a Self::Arena,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        data: &[u8],
    ) -> &'a Self::Image {
        // initial data specified, allocate a texture
        let raw = RawImage::new_texture(format, &dimensions, mipcount, samples);

        /*unsafe {
            upload_image_region(
                raw.target,
                raw.obj,
                format,
                0,
                (0, 0, 0),
                dimensions.width_height_depth(),
                data,
            );
        }*/

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
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a Self::Image {
        self.resources
            .lock()
            .unwrap()
            .alloc_aliased_image(arena, scope, format, dimensions, mipcount, samples, usage)
    }

    //----------------------------------------------------------------------------------------------

    /// Creates a framebuffer. See trait documentation for explanation of unsafety.
    fn create_framebuffer<'a>(
        &self,
        arena: &'a Self::Arena,
        color_attachments: &[renderer::Image<'a, Self>],
        depth_stencil_attachment: Option<renderer::Image<'a, Self>>,
    ) -> &'a Self::Framebuffer {
        arena
            .framebuffers
            .alloc(Framebuffer::new(color_attachments, depth_stencil_attachment).unwrap())
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
                .write(data, self.impl_params.uniform_buffer_alignment)
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
    fn create_buffer<'a>(&self, arena: &'a Self::Arena, size: u64) -> &'a Self::Buffer {
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
            //let obj = create_shader_from_spirv(stage, data_u32)
            //    .expect("failed to create shader from SPIR-V bytecode");
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
    ) -> &'a Self::GraphicsPipeline {
        create_graphics_pipeline_internal(arena, create_info)
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set_layout<'a>(
        &self,
        arena: &'a Self::Arena,
        bindings: &[DescriptorSetLayoutBinding],
    ) -> &'a Self::DescriptorSetLayout {
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
    ) -> &'a Self::DescriptorSet {
        let mut sampler_cache = self.sampler_cache.lock().unwrap();
        let descriptor_set =
            DescriptorSet::from_descriptors_and_layout(descriptors, layout, &mut sampler_cache);
        arena.descriptor_sets.alloc(descriptor_set)
    }

    //----------------------------------------------------------------------------------------------
    fn submit_frame<'a>(&self, frame: &[Command<'a, Self>]) {
        let mut resources = self.resources.lock().unwrap();
        let mut state_cache = self.state_cache.lock().unwrap();

        // execute commands
        {
            let mut execute_context = ExecuteContext::new(
                &mut resources,
                &mut state_cache,
                &self.window,
                &self.impl_params,
            );
            for cmd in frame.iter() {
                execute_context.execute_command(cmd);
            }
        }

        let mut frame_number = self.frame_number.lock().unwrap();
        let mut timeline = self.timeline.lock().unwrap();
        timeline.signal(*frame_number);

        // wait for previous frames before starting a new one
        // if max_frames_in_flight is zero, then will wait on the previously signalled point.
        if *frame_number > u64::from(self.max_frames_in_flight) {
            let timeout = !timeline.client_sync(
                *frame_number - u64::from(self.max_frames_in_flight),
                Timeout::Nanoseconds(1_000_000),
            );
            if timeout {
                panic!("timeout waiting for frame to finish")
            }
        }

        *frame_number += 1;
        // update default framebuffer size
        *self.default_swapchain.size.lock().unwrap() = self.window.get_inner_size().unwrap().into();
    }
}
