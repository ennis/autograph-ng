//! Images
use std::cell::Cell;
use std::ptr;

use ash::vk;

use alloc::{Allocator,Allocation,AllocationCreateInfo};
use handle::OwningHandle;
use sync::SyncGroup;
use context::{Context, FrameNumber, VkDevice1, FRAME_NONE};
use resource::Resource;

//--------------------------------------------------------------------------------------------------
// Image

pub trait ImageDescription
{
    fn dimensions(&self) -> Dimensions;
    fn format(&self) -> vk::Format;
    fn usage(&self) -> vk::ImageUsageFlags;
}

/// **Borrowed from vulkano**
#[derive(Copy,Clone,Debug)]
pub enum Dimensions
{
    Dim1d { width: u32 },
    Dim1dArray { width: u32, array_layers: u32 },
    Dim2d { width: u32, height: u32 },
    Dim2dArray { width: u32, height: u32, array_layers: u32 },
    Dim3d { width: u32, height: u32, depth: u32 },
    Cubemap { size: u32 },
    CubemapArray { size: u32, array_layers: u32 },
}

impl Dimensions {
    #[inline]
    pub fn width(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { width } => width,
            Dimensions::Dim1dArray { width, .. } => width,
            Dimensions::Dim2d { width, .. } => width,
            Dimensions::Dim2dArray { width, .. } => width,
            Dimensions::Dim3d { width, .. } => width,
            Dimensions::Cubemap { size } => size,
            Dimensions::CubemapArray { size, .. } => size,
        }
    }

    #[inline]
    pub fn height(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim1dArray { .. } => 1,
            Dimensions::Dim2d { height, .. } => height,
            Dimensions::Dim2dArray { height, .. } => height,
            Dimensions::Dim3d { height, .. } => height,
            Dimensions::Cubemap { size } => size,
            Dimensions::CubemapArray { size, .. } => size,
        }
    }

    #[inline]
    pub fn width_height(&self) -> [u32; 2] {
        [self.width(), self.height()]
    }

    #[inline]
    pub fn depth(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim1dArray { .. } => 1,
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim2dArray { .. } => 1,
            Dimensions::Dim3d { depth, .. } => depth,
            Dimensions::Cubemap { .. } => 1,
            Dimensions::CubemapArray { .. } => 1,
        }
    }

    #[inline]
    pub fn width_height_depth(&self) -> [u32; 3] {
        [self.width(), self.height(), self.depth()]
    }

    #[inline]
    pub fn array_layers(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim1dArray { array_layers, .. } => array_layers,
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim2dArray { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { .. } => 1,
            Dimensions::CubemapArray { array_layers, .. } => array_layers,
        }
    }

    #[inline]
    pub fn array_layers_with_cube(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim1dArray { array_layers, .. } => array_layers,
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim2dArray { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { .. } => 6,
            Dimensions::CubemapArray { array_layers, .. } => array_layers * 6,
        }
    }

    /*/// Builds the corresponding `ImageDimensions`.
    #[inline]
    pub fn to_image_dimensions(&self) -> ImageDimensions {
        match *self {
            Dimensions::Dim1d { width } => {
                ImageDimensions::Dim1d {
                    width: width,
                    array_layers: 1,
                }
            },
            Dimensions::Dim1dArray {
                width,
                array_layers,
            } => {
                ImageDimensions::Dim1d {
                    width: width,
                    array_layers: array_layers,
                }
            },
            Dimensions::Dim2d { width, height } => {
                ImageDimensions::Dim2d {
                    width: width,
                    height: height,
                    array_layers: 1,
                    cubemap_compatible: false,
                }
            },
            Dimensions::Dim2dArray {
                width,
                height,
                array_layers,
            } => {
                ImageDimensions::Dim2d {
                    width: width,
                    height: height,
                    array_layers: array_layers,
                    cubemap_compatible: false,
                }
            },
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => {
                ImageDimensions::Dim3d {
                    width: width,
                    height: height,
                    depth: depth,
                }
            },
            Dimensions::Cubemap { size } => {
                ImageDimensions::Dim2d {
                    width: size,
                    height: size,
                    array_layers: 6,
                    cubemap_compatible: true,
                }
            },
            Dimensions::CubemapArray { size, array_layers } => {
                ImageDimensions::Dim2d {
                    width: size,
                    height: size,
                    array_layers: array_layers * 6,
                    cubemap_compatible: true,
                }
            },
        }
    }*/

    /*/// Builds the corresponding `ViewType`.
    #[inline]
    pub fn to_view_type(&self) -> ViewType {
        match *self {
            Dimensions::Dim1d { .. } => ViewType::Dim1d,
            Dimensions::Dim1dArray { .. } => ViewType::Dim1dArray,
            Dimensions::Dim2d { .. } => ViewType::Dim2d,
            Dimensions::Dim2dArray { .. } => ViewType::Dim2dArray,
            Dimensions::Dim3d { .. } => ViewType::Dim3d,
            Dimensions::Cubemap { .. } => ViewType::Cubemap,
            Dimensions::CubemapArray { .. } => ViewType::CubemapArray,
        }
    }*/

    /// Returns the total number of texels for an image of these dimensions.
    #[inline]
    pub fn num_texels(&self) -> u32 {
        self.width() * self.height() * self.depth() * self.array_layers_with_cube()
    }
}

/// **Borrowed from vulkano**
/// Specifies how many mipmaps must be allocated.
///
/// Note that at least one mipmap must be allocated, to store the main level of the image.
#[derive(Debug, Copy, Clone)]
pub enum MipmapsCount {
    /// Allocates the number of mipmaps required to store all the mipmaps of the image where each
    /// mipmap is half the dimensions of the previous level. Guaranteed to be always supported.
    ///
    /// Note that this is not necessarily the maximum number of mipmaps, as the Vulkan
    /// implementation may report that it supports a greater value.
    Log2,

    /// Allocate one mipmap (ie. just the main level). Always supported.
    One,

    /// Allocate the given number of mipmaps. May result in an error if the value is out of range
    /// of what the implementation supports.
    Specific(u32),
}

impl From<u32> for MipmapsCount {
    #[inline]
    fn from(num: u32) -> MipmapsCount {
        MipmapsCount::Specific(num)
    }
}

/// Wrapper around vulkan images.
#[derive(Debug)]
pub struct Image {
    /// Name of the resource. May not uniquely identify the resource;
    name: String,

    /// Vulkan image object.
    image: OwningHandle<vk::Image>,

    /// Dimensions of the image.
    dimensions: Dimensions,

    /// Format of pixel data.
    format: vk::Format,

    /// Planned image usage.
    usage: vk::ImageUsageFlags,

    /// Associated memory allocation, `None` if not allocated by us (e.g. swapchain images).
    memory: Option<Allocation>,

    /// Last known layout.
    last_layout: Cell<vk::ImageLayout>,

    /// The frame on which the image was last used (bound to the pipeline).
    last_used: FrameNumber,

    /// If the image is part of a swapchain, that's its index. Otherwise, None.
    swapchain_index: Option<u32>,

    /// Used for synchronization between frames.
    exit_semaphores: SyncGroup<Vec<vk::Semaphore>>,
}

impl Image {

    /// Creates a new image resource, and allocate device memory for it on a suitable pool.
    pub fn new(context: &Context,
               name: impl Into<String>,
               dimensions: Dimensions,
               mipmaps_count: MipmapsCount,
               usage: vk::ImageUsageFlags,
               queue_flags: vk::QueueFlags) -> Image
    {
    }

    /// Creates a new image and an associated allocation.
    pub(crate) fn new_unallocated(
        vkd: &VkDevice1,
        name: impl Into<String>,
        dimensions: Dimensions,
        mip_map_count: MipmapsCount,
        usage: vk::ImageUsageFlags,
        queue_families: &[u32],
        preferred_memory_flags: vk::MemoryPropertyFlags,
        required_memory_flags: vk::MemoryPropertyFlags,
        allocator: &mut Allocator) -> Image
    {
        unsafe {
            let image = vkd.create_image(&create_info, None).expect("could not create image");
            let memory_requirements = vkd.get_image_memory_requirements(image);
            let allocation = allocator.allocate_memory(&AllocationCreateInfo {
                size: memory_requirements.size,
                alignment: memory_requirements.alignment,
                memory_type_bits: memory_requirements.memory_type_bits,
                required_flags: required_memory_flags,
                preferred_flags: preferred_memory_flags,
            }, vkd).expect("could not allocate memory for the image");
            vkd.bind_image_memory(image, alloc.device_memory, alloc.range.start).expect("failed to bind image memory");

            Image {
                name: name.into(),
                desc: ImageDescription {
                    initial_layout: create_info.initial_layout,
                    extent: create_info.extent,
                    samples: create_info.samples,
                    flags: create_info.flags,
                    image_type: create_info.image_type,
                    array_layers: create_info.array_layers,
                    sharing_mode: create_info.sharing_mode,
                    usage: create_info.usage,
                    format: create_info.format,
                    mip_levels: 0,
                    tiling: create_info.tiling,
                    queue_flags,
                },
                image,
                swapchain_index: None,
                last_layout: Cell::new(vk::ImageLayout::General),
                last_used: FRAME_NONE,
                exit_semaphores: SyncGroup::new(),
                allocation,
            }
        }

    }

    /// Creates a new image for the specified swapchain image.
    pub(crate) fn new_swapchain_image(
        name: impl Into<String>,
        swapchain_create_info: &vk::SwapchainCreateInfoKHR,
        image: OwningHandle<vk::Image>,
        swapchain_index: u32,
    ) -> Image {
        Image {
            name: name.into(),
            create_info: vk::ImageCreateInfo {
                s_type: vk::StructureType::ImageCreateInfo,
                p_next: ptr::null(),
                flags: vk::ImageCreateFlags::empty(),
                image_type: vk::ImageType::Type2d,
                format: swapchain_create_info.image_format,
                extent: vk::Extent3D {
                    width: swapchain_create_info.image_extent.width,
                    height: swapchain_create_info.image_extent.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: swapchain_create_info.image_array_layers,
                samples: vk::SAMPLE_COUNT_1_BIT,
                tiling: vk::ImageTiling::Optimal,
                usage: swapchain_create_info.image_usage,
                sharing_mode: swapchain_create_info.image_sharing_mode,
                queue_family_index_count: 0,
                p_queue_family_indices: ptr::null(),
                initial_layout: vk::ImageLayout::Undefined,
            },
            image: Some(image),
            swapchain_index: Some(swapchain_index),
            last_layout: Cell::new(vk::ImageLayout::Undefined),
            last_used: FRAME_NONE,
            exit_semaphores: SyncGroup::new(),
        }
    }

    /// Sets a list of semaphores signalled by the resource when the frame ends.
    pub(crate) fn set_exit_semaphores(
        &mut self,
        semaphores: Vec<vk::Semaphore>,
        frame_sync: &mut FrameSync,
        vkd: &VkDevice1,
    ) {
        self.exit_semaphores
            .enqueue(semaphores, frame_sync, |semaphores| {
                for sem in semaphores {
                    unsafe {
                        vkd.destroy_semaphore(sem, None);
                    }
                }
            });
    }

    /// Returns the dimensions of the image.
    pub fn dimensions(&self) -> (u32, u32, u32) {
        (
            self.create_info.extent.width,
            self.create_info.extent.height,
            self.create_info.extent.depth,
        )
    }

    /// Returns the format of the image.
    pub fn format(&self) -> vk::Format {
        self.create_info.format
    }

    /// Returns the usage flags of the image.
    pub fn usage(&self) -> vk::ImageUsageFlags {
        self.create_info.usage
    }

    pub(crate) fn is_allocated(&self) -> bool {
        self.image.is_some()
    }

    pub(crate) fn is_swapchain_image(&self) -> bool {
        self.swapchain_index.is_some()
    }

    pub(crate) fn last_layout(&self) -> vk::ImageLayout {
        self.last_layout.get()
    }
}


impl ImageDescription for Image
{
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn format(&self) -> vk::Format {
        self.format
    }

    fn usage(&self) -> vk::ImageUsageFlags {
        self.usage
    }
}

impl Resource for Image {
    type CreateInfo = vk::ImageCreateInfo;

    fn name(&self) -> &str {
        &self.name
    }

    fn last_used_frame(&self) -> FrameNumber {
        self.last_used
    }

    fn create_info(&self) -> &vk::ImageCreateInfo {
        &self.create_info
    }
}
