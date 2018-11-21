use glutin::{GlContext, GlWindow};
use std::collections::HashMap;
use std::ffi::CStr;
use std::mem;
use std::os::raw::{c_char, c_void};
use std::slice;
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use std::ops::Range;
use std::cmp::{min, max};

mod api;
//mod sampler;
mod buffer;
mod format;
mod image;
mod upload;
mod window;
mod sync;

use config::Config;
//use sid_vec::{FromIndex, Id, IdVec};
use slotmap::SlotMap;
use smallvec::SmallVec;

use self::api as gl;
use self::api::types::*;
use self::image::{upload_image_region, Image};
use self::upload::{MappedBufferRange, MappedBufferRangeStack, MultiBuffer};
use self::sync::Timeline;

use crate::renderer::*;

pub use self::window::create_backend_and_window;

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
struct GlObject<T> {
    /// Handle
    obj: T,
    /// Pending uses in frame
    pending_uses: u64,
    /// Should be deleted or recycled once free
    marked_for_deletion: bool,
}

/// Copy + Clone to bypass a restriction of slotmap on stable rust.
#[derive(Copy,Clone,Debug)]
struct Buffer
{
    obj: GLuint,
    shared: bool,
    offset: usize,
    size: usize,
}

//--------------------------------------------------------------------------------------------------
struct GlImplementationDetails
{
    uniform_buffer_alignment: usize,
}

new_key_type! {
    pub struct ImageHandle;
    pub struct BufferHandle;
    pub struct DescriptorSetHandle;
    pub struct DescriptorSetLayoutHandle;
    pub struct GraphicsPipelineHandle;
}

pub struct DescriptorSetLayout
{
    bindings: Vec<LayoutBinding>,
}


const MAX_RESOURCES_PER_SET: usize = 8;

// The concept of descriptor sets does not exist in OpenGL.
// We emulate them by mapping a descriptor set to a range of binding locations.
// e.g. 0 => binding 0..4, 1 => binding 5..10, etc.
// These ranges of locations are shared across every kind of binding (uniform buffers, images, textures).
//

pub struct DescriptorSet
{
    /*textures: [GLuint; MAX_RESOURCES_PER_SET],
    samplers: [GLuint; MAX_RESOURCES_PER_SET],
    images: [GLuint; MAX_RESOURCES_PER_SET],
    uniform_buffers: [GLuint; MAX_RESOURCES_PER_SET],
    uniform_buffer_sizes: [GLsizeiptr; MAX_RESOURCES_PER_SET],
    uniform_buffer_offsets: [GLintptr; MAX_RESOURCES_PER_SET],
    shader_storage_buffers: [GLuint; MAX_RESOURCES_PER_SET],
    shader_storage_buffer_sizes: [GLsizeiptr; MAX_RESOURCES_PER_SET],
    shader_storage_buffer_offsets: [GLintptr; MAX_RESOURCES_PER_SET],*/
}

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

//--------------------------------------------------------------------------------------------------
pub struct OpenGlBackendInner
{
    images: SlotMap<ImageHandle, Image>,
    buffers: SlotMap<BufferHandle, Buffer>,
    descriptor_set_layouts: SlotMap<DescriptorSetLayoutHandle, DescriptorSetLayout>,
    descriptor_sets: SlotMap<DescriptorSetHandle, DescriptorSet>,
    frame_idx: u64,
    timeline: Timeline,
    upload_buf: MultiBuffer,
    upload_range: MappedBufferRangeStack,
}

pub struct OpenGlBackend {
    //cache: Cache,
    //sampler_cache: Mutex<HashMap<SamplerDesc, Sampler>>,
    impl_details: GlImplementationDetails,
    window: GlWindow,
    inner: Mutex<OpenGlBackendInner>
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
            gl::DebugMessageCallback(debug_callback as GLDEBUGPROC, 0 as *const c_void);
            gl::DebugMessageControl(
                gl::DONT_CARE,
                gl::DONT_CARE,
                gl::DONT_CARE,
                0,
                0 as *const u32,
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

        let uniform_buffer_alignment = unsafe {
            let mut v = mem::uninitialized();
            gl::GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut v);
            v as usize
        };

        let mut timeline = Timeline::new(0);
        let mut upload_buf = MultiBuffer::new(upload_buffer_size as usize);
        let upload_range = MappedBufferRangeStack::new(upload_buf.acquire_buffer_range(1, &mut timeline));

        OpenGlBackend {
            //cache: Cache::new(),
            //sampler_cache: Mutex::new(HashMap::new()),
            inner: Mutex::new(OpenGlBackendInner {
                images: SlotMap::with_key(),
                buffers: SlotMap::with_key(),
                descriptor_set_layouts: SlotMap::with_key(),
                descriptor_sets: SlotMap::with_key(),
                frame_idx: 1,
                timeline,
                upload_buf,
                upload_range,
            }),
            window,
            impl_details: GlImplementationDetails {
                uniform_buffer_alignment
            },
        }
    }
}

/*pub struct ImageHandleTag;
pub struct BufferHandleTag;
pub struct SwapchainHandleTag;
pub struct DescriptorSetHandleTag;
pub struct DescriptorSetLayoutHandleTag;
pub struct GraphicsPipelineHandleTag;*/

impl RendererBackend for OpenGlBackend {
    type SwapchainHandle = usize;
    type BufferHandle = BufferHandle;
    type ImageHandle = ImageHandle;
    type DescriptorSetHandle = DescriptorSetHandle;
    type DescriptorSetLayoutHandle = DescriptorSetLayoutHandle;
    type GraphicsPipelineHandle = GraphicsPipelineHandle;

    fn create_swapchain(&self) -> Self::SwapchainHandle {
        unimplemented!()
    }

    fn default_swapchain(&self) -> Option<Self::SwapchainHandle> {
        Some(0)
    }

    fn swapchain_dimensions(&self, swapchain: Self::SwapchainHandle) -> (u32, u32) {
        assert_eq!(swapchain, 0, "invalid swapchain handle");
        self.window.get_inner_size().unwrap().into()
    }

    fn create_image(
        &self,
        format: Format,
        dimensions: &Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        usage: ImageUsageFlags,
        initial_data: Option<&[u8]>,
    ) -> Self::ImageHandle {
        let img = if let Some(data) = initial_data {
            // initial data specified, allocate a texture
            let img = Image::new_texture(format, dimensions, mipcount, samples);
            unsafe {
                upload_image_region(
                    &img,
                    format,
                    0,
                    (0, 0, 0),
                    dimensions.width_height_depth(),
                    data,
                );
            }
            img
        } else if usage.contains(ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLE) {
            // will be used as storage or sampled image
            Image::new_texture(format, dimensions, mipcount, samples)
        } else {
            // only used as color attachments: can use a renderbuffer instead
            Image::new_renderbuffer(format, dimensions, samples)
        };

        self.inner.lock().unwrap().images.insert(img)
    }

    fn upload_transient(&self, data: &[u8]) -> BufferSlice<Self::BufferHandle> {
        // acquire mapped buffer range for current frame if not already done
        // write data at current pointer
        // flush
        let mut inner = self.inner.lock().unwrap();
        inner.upload_range.write(data, self.impl_details.uniform_buffer_alignment).expect("unable to upload data");
        inner.upload_range.flush();   // XXX not necessary to make it visible already
        unimplemented!()
    }

    fn destroy_image(&self, image: Self::ImageHandle) {
        // delete the image right now, since OpenGL will handle the actual resource deletion
        // once the resource is not used anymore.
        let mut inner = self.inner.lock().unwrap();
        let obj = inner.images[image].obj;
        unsafe {
            gl::DeleteTextures(1, &obj);
        }
    }

    fn create_buffer(&self, size: u64) -> Self::BufferHandle {
        unimplemented!()
    }

    fn destroy_buffer(&self, buffer: Self::BufferHandle) {
        unimplemented!()
    }

    fn submit_frame(&self) {
        //
        let mut inner = self.inner.lock().unwrap();

        let idx = inner.frame_idx;
        inner.timeline.signal(idx);
        inner.frame_idx += 1;

        unimplemented!()
    }

    fn create_graphics_pipeline<'a>(&self, shaders: &GraphicsShaderPipeline<'a>) -> Self::GraphicsPipelineHandle {
        unimplemented!()
    }

    fn create_descriptor_set_layout(&self, bindings: &[LayoutBinding]) -> Self::DescriptorSetLayoutHandle {
        assert_ne!(bindings.len(), 0, "descriptor set layout has no bindings");
        let mut inner = self.inner.lock().unwrap();

        inner.descriptor_set_layouts.insert(DescriptorSetLayout {
            bindings: bindings.to_vec(),
        })
    }

    fn create_descriptor_set(&self, layout: Self::DescriptorSetLayoutHandle, descriptors: &[Descriptor<Self>]) -> Self::DescriptorSetHandle {
        // convert the descriptor set to a set of uniform and textures
        let mut inner = self.inner.lock().unwrap();
        let layout = &inner.descriptor_set_layouts[layout];
        let mut ds = DescriptorSet::new();

        for (i,d) in descriptors.iter().enumerate() {
            let layout_entry = layout.bindings[i];

            match layout_entry.descriptor_type {
                DescriptorType::SampledImage => {
                    if let &Descriptor::SampledImage { img, sampler } = d {

                    } else {
                        // wrong type
                        warn!("descriptor #{} does not match corresponding layout entry (expected: SampledImage)", i);
                    }
                },
                DescriptorType::UniformBuffer => {

                },
                DescriptorType::StorageImage => {},
            }
        }

        unimplemented!()
    }
}
