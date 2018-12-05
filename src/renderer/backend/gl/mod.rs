use glutin::{GlContext, GlWindow};
use std::ffi::CStr;
use std::mem;
use std::cell::Cell;
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
// Resource cache
mod cache;
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
use self::buffer::*;
use self::cache::*;
use self::format::*;
use self::image::*;
pub use self::pipeline::GraphicsPipelineCreateInfoAdditional;
use self::pipeline::*;
pub use self::pipeline_file::PipelineDescriptionFile;
use self::shader::*;
use self::state::*;
use self::sync::*;
use self::upload::*;
pub use self::window::create_backend_and_window;
use crate::renderer::command_buffer::*;
use crate::renderer;
use crate::renderer::util;
use crate::renderer::*;

//--------------------------------------------------------------------------------------------------
extern "system" fn debug_callback(
    _source: GLenum,
    _ty: GLenum,
    _id: GLuint,
    _severity: GLenum,
    length: GLsizei,
    msg: *const GLchar,
    _data: *mut GLvoid,
) {
    let str = unsafe {
        str::from_utf8(slice::from_raw_parts(msg as *const u8, length as usize)).unwrap()
    };
    debug!("(GL) {}", str);
}

//--------------------------------------------------------------------------------------------------
struct FrameBoundObject<T> {
    /// Handle
    obj: T,
    /// Pending uses in frame
    pending_uses: u64,
    /// Should be deleted or recycled once free
    marked_for_deletion: bool,
}

//--------------------------------------------------------------------------------------------------
pub struct ImplementationParameters {
    pub uniform_buffer_alignment: usize,
    pub max_draw_buffers: u32,
    pub max_color_attachments: u32,
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
        }
    }
}

pub struct DescriptorSetLayout {
    bindings: Vec<LayoutBinding>,
}

pub struct AttachmentLayout {
    attachments: Vec<AttachmentDescription>,
}

const MAX_RESOURCES_PER_SET: usize = 8;

// The concept of descriptor sets does not exist in OpenGL.
// We emulate them by mapping a descriptor set to a range of binding locations.
// e.g. 0 => binding 0..4, 1 => binding 5..10, etc.
// These ranges of locations are shared across every kind of binding (uniform buffers, images, textures).
//

pub struct DescriptorSet {
    /*textures: [GLuint; MAX_RESOURCES_PER_SET],
samplers: [GLuint; MAX_RESOURCES_PER_SET],
images: [GLuint; MAX_RESOURCES_PER_SET],
uniform_buffers: [GLuint; MAX_RESOURCES_PER_SET],
uniform_buffer_sizes: [GLsizeiptr; MAX_RESOURCES_PER_SET],
uniform_buffer_offsets: [GLintptr; MAX_RESOURCES_PER_SET],
shader_storage_buffers: [GLuint; MAX_RESOURCES_PER_SET],
shader_storage_buffer_sizes: [GLsizeiptr; MAX_RESOURCES_PER_SET],
shader_storage_buffer_offsets: [GLintptr; MAX_RESOURCES_PER_SET],*/}

impl DescriptorSet {
    fn new() -> DescriptorSet {
        DescriptorSet {
            /*textures: SmallVec::new(),
            samplers: SmallVec::new(),
            images: SmallVec::new(),
            uniform_buffers: SmallVec::new(),
            uniform_buffer_sizes: SmallVec::new(),
            uniform_buffer_offsets: SmallVec::new(),
            shader_storage_buffers: SmallVec::new(),
            shader_storage_buffer_sizes: SmallVec::new(),
            shader_storage_buffer_offsets: SmallVec::new()*/
        }
    }
}

/*//--------------------------------------------------------------------------------------------------
new_key_type! {
    pub struct ImageHandle;
    pub struct BufferHandle;
    pub struct DescriptorSetHandle;
    pub struct DescriptorSetLayoutHandle;
    pub struct ShaderModuleHandle;
    pub struct GraphicsPipelineHandle;
    pub struct TransientImageCacheKey;
    pub struct TransientBufferCacheKey;
}*/

//--------------------------------------------------------------------------------------------------

struct Swapchain
{
    size: Cell<(u32,u32)>,
}

impl renderer::Swapchain for Swapchain
{
    fn size(&self) -> (u32, u32) {
        self.size.get()
    }
}

impl renderer::Buffer for Buffer {}
impl renderer::Image for Image {}
impl renderer::DescriptorSet for DescriptorSet {}

pub struct Arena
{
    pub swapchains: util::SyncArena<Swapchain>,
    pub buffers: util::SyncArena<Buffer>,
    pub images: util::SyncArena<Image>,
    pub descriptor_sets: util::SyncArena<DescriptorSet>,
    pub descriptor_set_layouts: util::SyncArena<DescriptorSetLayout>,
    pub shader_modules: util::SyncArena<ShaderModule>,
    pub graphics_pipelines: util::SyncArena<GraphicsPipeline>
}


pub struct OpenGlBackendInner {
    target_size: (u32, u32),
    image_cache: Vec<Image>,
    frame_idx: u64,
    timeline: Timeline,
    upload_buf: MultiBuffer,
    upload_range: MappedBufferRangeStack,
    state_cache: StateCache,
}

pub struct OpenGlBackend {
    //cache: Cache,
    //sampler_cache: Mutex<HashMap<SamplerDesc, Sampler>>,
    //fbo_cache
    inner: Mutex<OpenGlBackendInner>,
    arena: Arena,
    impl_params: ImplementationParameters,
    window: GlWindow,
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
        let mut upload_buf = MultiBuffer::new(upload_buffer_size as usize);
        let upload_range =
            MappedBufferRangeStack::new(upload_buf.acquire_buffer_range(1, &mut timeline));

        let impl_params = ImplementationParameters::populate();
        let state_cache = StateCache::new(&impl_params);

        OpenGlBackend {
            //cache: Cache::new(),
            //sampler_cache: Mutex::new(HashMap::new()),
            inner: Mutex::new(OpenGlBackendInner {
                images: ResourceCache::new(),
                buffers: SlotMap::with_key(),
                state_cache,
                descriptor_set_layouts: SlotMap::with_key(),
                descriptor_sets: SlotMap::with_key(),
                graphics_pipelines: SlotMap::with_key(),
                shader_modules: SlotMap::with_key(),
                frame_idx: 1,
                timeline,
                upload_buf,
                upload_range,
                target_size: window.get_inner_size().unwrap().into(),
            }),
            window,
            max_frames_in_flight,
            impl_params,
        }
    }
}

// TODO move this into a function in the spirv module
const SPIRV_MAGIC: u32 = 0x0723_0203;

impl RendererBackend for OpenGlBackend {
    type Arena = Arena;
    type Swapchain = Swapchain;
    type Buffer = Buffer;
    type Image = Image;
    type DescriptorSet = DescriptorSet;
    type DescriptorSetLayout = DescriptorSetLayout;
    type ShaderModule = ShaderModule;
    type GraphicsPipeline = GraphicsPipeline;
    type GraphicsPipelineCreateInfoAdditional = GraphicsPipelineCreateInfoAdditional;
    //type AttachmentLayoutHandle = AttachmentLayoutHandle;

    fn create_arena(&self) -> Self::Arena {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    fn create_swapchain<'a>(&self, arena: &'a Self::Arena) -> &'a Self::Swapchain
    {
        unimplemented!()
    }

    fn default_swapchain<'rcx>(&'rcx self) -> Option<&'rcx Self::Swapchain>
    {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    fn create_image<'a>(
        &self,
        arena: &'a Self::Arena,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: Option<&[u8]>,
    ) -> &'a Self::Image
    {
        let img = if let Some(data) = initial_data {
            // initial data specified, allocate a texture
            let img = Image::new_texture(format, &dimensions, mipcount, samples);
            unsafe {
                upload_image_region(
                    img.target,
                    img.obj,
                    format,
                    0,
                    (0, 0, 0),
                    dimensions.width_height_depth(),
                    data,
                );
            }
            img
        } else if usage.intersects(ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLE) {
            // will be used as storage or sampled image
            Image::new_texture(format, &dimensions, mipcount, samples)
        } else {
            // only used as color attachments: can use a renderbuffer instead
            Image::new_renderbuffer(format, &dimensions, samples)
        };

        arena.images.alloc(img)
    }

    //----------------------------------------------------------------------------------------------
    fn create_scoped_image<'a>(
        &self,
        arena: &'a Self::Arena,
        scope: Scope,
        format: Format,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> &'a Self::Image {
        let create_info = ImageCreateInfo::new(format, dimensions, mipcount, samples, usage);
        let mut inner = self.inner.lock().unwrap();
        let key = inner.images.create_scoped(scope, create_info);
        let frame = inner.frame_idx;
        inner.images.allocate_scoped(key, frame, |c| {
            debug!(
                "Allocating new scoped image {:?} ({:?}, {:?}, mips: {}, samples: {})",
                c.dimensions, c.format, c.usage, c.mipcount, c.samples
            );
            if c.usage
                .intersects(ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLE)
            {
                // will be used as storage or sampled image
                Image::new_texture(
                    c.format,
                    &c.dimensions,
                    MipmapsCount::Specific(c.mipcount),
                    samples,
                )
            } else {
                // only used as color attachments: can use a renderbuffer instead
                Image::new_renderbuffer(c.format, &c.dimensions, c.samples)
            }
        });
        key
    }

    fn create_shader_module<'a>(
        &self,
        arena: &'a Self::Arena,
        data: &[u8],
        stage: ShaderStageFlags,
    ) -> &'a Self::ShaderModule
    {
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

    /*
    fn destroy_shader_module(&self, module: Self::ShaderModuleHandle) {
        let mut inner = self.inner.lock().unwrap();
        inner.shader_modules.remove(module);
    }*/

    //----------------------------------------------------------------------------------------------
    /*fn upload_transient(&self, data: &[u8]) -> BufferSlice<Self::BufferHandle> {
        // acquire mapped buffer range for current frame if not already done
        // write data at current pointer
        // flush
        let mut inner = self.inner.lock().unwrap();
        let offset = inner
            .upload_range
            .write(data, self.impl_params.uniform_buffer_alignment)
            .expect("unable to upload data");
        inner.upload_range.flush(); // XXX not necessary to make it visible already
        unimplemented!()
        /*BufferSlice {
            buffer: inner.upload_range.buffer(),
            offset,
            size: data.len()
        }*/
    }*/

    //----------------------------------------------------------------------------------------------
    /*fn destroy_image(&self, image: Self::ImageHandle) {
        // delete the image right now, since OpenGL will handle the actual resource deletion
        // once the resource is not used anymore.
        let mut inner = self.inner.lock().unwrap();
        inner.images.destroy(image, |_| {});
    }*/

    //----------------------------------------------------------------------------------------------
    fn create_buffer<'a>(&self,
                         arena: &'a Self::Arena,
                         size: u64) -> &'a Self::Buffer
    {
        unimplemented!()
    }


    //----------------------------------------------------------------------------------------------
    fn submit_frame(&self, frame: SubmitFrame<Self>) {
        //
        let mut inner = self.inner.lock().unwrap();

        // execute commands
        {
            let mut execute_context =
                ExecuteContext::new(inner.deref_mut(), &self.window, &self.impl_params);
            for cmd in frame.commands.iter() {
                execute_context.execute_command(cmd);
            }
        }

        let idx = inner.frame_idx;
        inner.timeline.signal(idx);

        // wait for previous frames before starting a new one
        // if max_frames_in_flight is zero, then will wait on the previously signalled point.
        if idx > u64::from(self.max_frames_in_flight) {
            let timeout = !inner.timeline.client_sync(
                idx - u64::from(self.max_frames_in_flight),
                Timeout::Nanoseconds(1_000_000),
            );
            if timeout {
                panic!("timeout waiting for frame to finish")
            }
        }

        inner.frame_idx += 1;
        // update default framebuffer size
        inner.target_size = self.window.get_inner_size().unwrap().into();
    }

    //----------------------------------------------------------------------------------------------
    fn create_graphics_pipeline<'a>(
        &self,
        arena: &'a Self::Arena,
        create_info: &GraphicsPipelineCreateInfo<'a, Self>,
    ) -> &'a Self::GraphicsPipeline {
        self.create_graphics_pipeline_internal(create_info)
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set_layout<'a>(
        &self,
        arena: &'a Self::Arena,
        bindings: &[LayoutBinding],
    ) -> &'a Self::DescriptorSetLayout {
        assert_ne!(bindings.len(), 0, "descriptor set layout has no bindings");
        let mut inner = self.inner.lock().unwrap();

        inner.descriptor_set_layouts.insert(DescriptorSetLayout {
            bindings: bindings.to_vec(),
        })
    }

    //----------------------------------------------------------------------------------------------
    fn create_descriptor_set<'a>(
        &self,
        arena: &'a Self::Arena,
        layout: Self::DescriptorSetLayoutHandle,
        descriptors: &[Descriptor<Self>],
    ) -> &'a Self::DescriptorSet {
        // convert the descriptor set to a set of uniform and textures
        let mut inner = self.inner.lock().unwrap();
        let layout = &inner.descriptor_set_layouts[layout];
        let mut ds = DescriptorSet::new();

        for (i, d) in descriptors.iter().enumerate() {
            let layout_entry = layout.bindings[i];

            match layout_entry.descriptor_type {
                DescriptorType::SampledImage => {
                    if let &Descriptor::SampledImage { img, sampler } = d {

                    } else {
                        // wrong type
                        warn!("descriptor #{} does not match corresponding layout entry (expected: SampledImage)", i);
                    }
                }
                DescriptorType::UniformBuffer => {}
                DescriptorType::StorageImage => {}
                _ => unimplemented!(),
            }
        }

        unimplemented!()
    }

    fn drop_arena(&self, arena: <Self as RendererBackend>::Arena) where Self: Sized {
        unimplemented!()
    }

    /*fn create_attachment_layout(&self, attachments: &[AttachmentDescription]) -> Self::AttachmentLayoutHandle {
        let mut inner = self.inner.lock().unwrap();
        inner.attachment_layouts.insert(AttachmentLayout { attachments: attachments.to_vec() })
    }*/
}

struct ExecuteContext<'a> {
    backend: &'a mut OpenGlBackendInner,
    window: &'a GlWindow,
    impl_details: &'a ImplementationParameters,
}

impl<'a> ExecuteContext<'a> {
    fn new(
        backend: &'a mut OpenGlBackendInner,
        window: &'a GlWindow,
        impl_details: &'a ImplementationParameters,
    ) -> ExecuteContext<'a> {
        ExecuteContext {
            backend,
            window,
            impl_details,
        }
    }

    fn cmd_clear_image_float(&mut self, image: ImageHandle, color: &[f32; 4]) {
        let img = &self.backend.images.get(image).unwrap();
        let obj = img.obj;
        if img.target == gl::RENDERBUFFER {
            // create temporary framebuffer
            let mut tmpfb = 0;
            unsafe {
                gl::CreateFramebuffers(1, &mut tmpfb);
                gl::NamedFramebufferRenderbuffer(
                    tmpfb,
                    gl::COLOR_ATTACHMENT0,
                    gl::RENDERBUFFER,
                    img.obj,
                );
                gl::NamedFramebufferDrawBuffers(tmpfb, 1, (&[gl::COLOR_ATTACHMENT0]).as_ptr());
                gl::ClearNamedFramebufferfv(tmpfb, gl::COLOR, 0, color.as_ptr());
                gl::DeleteFramebuffers(1, &tmpfb);
            }
        } else {
            // TODO specify which level to clear in command
            unsafe {
                gl::ClearTexImage(obj, 0, gl::RGBA, gl::FLOAT, color.as_ptr() as *const _);
            }
        }
    }

    fn cmd_clear_depth_stencil_image(
        &mut self,
        image: ImageHandle,
        depth: f32,
        stencil: Option<u8>,
    ) {
        let img = &self.backend.images.get(image).unwrap();
        let obj = img.obj;
        if img.target == gl::RENDERBUFFER {
            // create temporary framebuffer
            let mut tmpfb = 0;
            unsafe {
                gl::CreateFramebuffers(1, &mut tmpfb);
                gl::NamedFramebufferRenderbuffer(
                    tmpfb,
                    gl::DEPTH_ATTACHMENT,
                    gl::RENDERBUFFER,
                    img.obj,
                );
                if let Some(stencil) = stencil {
                    unimplemented!()
                } else {
                    gl::ClearNamedFramebufferfv(tmpfb, gl::DEPTH, 0, &depth);
                }
                gl::DeleteFramebuffers(1, &tmpfb);
            }
        } else {
            // TODO specify which level to clear in command
            unsafe {
                if let Some(stencil) = stencil {
                    unimplemented!()
                } else {
                    gl::ClearTexImage(
                        obj,
                        0,
                        gl::DEPTH_COMPONENT,
                        gl::FLOAT,
                        &depth as *const f32 as *const _,
                    );
                }
            }
        }
    }

    fn cmd_present(
        &mut self,
        image: ImageHandle,
        swapchain: <OpenGlBackend as RendererBackend>::SwapchainHandle,
    ) {
        // only handle default swapchain for now
        assert_eq!(swapchain, 0, "invalid swapchain handle");
        // make a framebuffer and bind the image to it

        unsafe {
            let mut tmpfb = 0;
            gl::CreateFramebuffers(1, &mut tmpfb);
            // bind image to it
            let img = self.backend.images.get(image).unwrap();
            if img.target == gl::RENDERBUFFER {
                gl::NamedFramebufferRenderbuffer(
                    tmpfb,
                    gl::COLOR_ATTACHMENT0,
                    gl::RENDERBUFFER,
                    img.obj,
                );
            } else {
                // TODO other levels / layers?
                gl::NamedFramebufferTexture(tmpfb, gl::COLOR_ATTACHMENT0, img.obj, 0);
            }
            // blit to default framebuffer
            //gl::BindFramebuffer(gl::READ_FRAMEBUFFER, tmpfb);
            let (w, h) = self.backend.target_size;

            gl::BlitNamedFramebuffer(
                tmpfb,
                0,
                0,        // srcX0
                0,        // srcY0
                w as i32, // srcX1,
                h as i32, // srcY1,
                0,        // dstX0,
                0,        // dstY0,
                w as i32, // dstX1
                h as i32, // dstY1
                gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT,
                gl::NEAREST,
            );

            // destroy temp framebuffer
            gl::DeleteFramebuffers(1, &tmpfb);
        }

        // swap buffers
        self.window.swap_buffers().expect("swap_buffers error")
    }

    fn execute_command(&mut self, command: &Command<OpenGlBackend>) {
        match command.cmd {
            CommandInner::PipelineBarrier {} => {
                // no-op on GL
            }
            CommandInner::AllocImage { image } => unimplemented!(),
            CommandInner::AllocBuffer { buffer } => unimplemented!(),
            CommandInner::DropImage { image } => unimplemented!(),
            CommandInner::DropBuffer { buffer } => unimplemented!(),
            CommandInner::SwapImages { a, b } => unimplemented!(),
            CommandInner::SwapBuffers { a, b } => unimplemented!(),
            CommandInner::ClearImageFloat { image, color } => {
                self.cmd_clear_image_float(image, &color);
            }
            CommandInner::ClearDepthStencilImage {
                image,
                depth,
                stencil,
            } => {
                self.cmd_clear_depth_stencil_image(image, depth, stencil);
            }
            CommandInner::Draw {} => unimplemented!(),
            CommandInner::Present { image, swapchain } => {
                self.cmd_present(image, swapchain);
            }
        }
    }
}



