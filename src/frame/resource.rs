use super::*;

//--------------------------------------------------------------------------------------------------
/// Identifies an image in the frame resource table.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ImageId(pub(crate) u32);

/// Identifies a buffer in the frame resource table.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BufferId(pub(crate) u32);

/// A special type of resource reference that identifies an image resource used as an attachment
/// between subpasses.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AttachmentId {
    pub(crate) img: ImageId,
    pub(crate) renderpass: RenderPassId,
    pub(crate) index: u32,
}

//--------------------------------------------------------------------------------------------------
/// A resource (image or buffer) used in a frame.
pub enum FrameResource<'imp, T: Resource, D> {
    Imported {
        resource: &'imp T,
    },
    Transient {
        name: String,
        description: D,
        resource: Option<T>,
    },
}

impl<'imp, T: Resource, D> FrameResource<'imp, T, D> {
    pub(crate) fn name(&self) -> &str {
        match self {
            FrameResource::Imported { resource } => resource.name(),
            FrameResource::Transient { ref name, .. } => name,
        }
    }

    pub(crate) fn is_imported(&self) -> bool {
        match self {
            FrameResource::Imported { .. } => true,
            _ => false,
        }
    }

    pub fn new_transient(name: String, description: D) -> FrameResource<'imp, T, D> {
        FrameResource::Transient {
            name,
            description,
            resource: None,
        }
    }

    pub fn new_imported(resource: &'imp T) -> FrameResource<'imp, T, D> {
        FrameResource::Imported { resource }
    }

    pub fn get_description_mut(&mut self) -> Option<&mut D> {
        match self {
            FrameResource::Transient {
                ref mut description,
                ..
            } => Some(description),
            _ => None,
        }
    }
}

//--------------------------------------------------------------------------------------------------
/*pub(crate) struct ImageDesc {
    pub(crate) flags: vk::ImageCreateFlags,
    pub(crate) image_type: vk::ImageType,
    pub(crate) format: vk::Format,
    pub(crate) extent: vk::Extent3D,
    pub(crate) mip_levels: u32,
    pub(crate) array_layers: u32,
    pub(crate) samples: vk::SampleCountFlags,
    pub(crate) tiling: vk::ImageTiling,
    pub(crate) usage: vk::ImageUsageFlags, // inferred
                                           //pub(crate) sharing_mode: SharingMode,
                                           //pub(crate) queue_family_index_count: uint32_t,    // inferred
                                           //pub(crate) p_queue_family_indices: *const uint32_t,
                                           //pub(crate) initial_layout: ImageLayout,   // inferred
}

pub(crate) struct BufferDesc {
    pub(crate) flags: vk::BufferCreateFlags,
    pub(crate) size: vk::DeviceSize,
    pub(crate) usage: vk::BufferUsageFlags,
    //pub(crate) sharing_mode: vk::SharingMode,
    //pub(crate) queue_family_index_count: uint32_t,
    //pub(crate) p_queue_family_indices: *const uint32_t,
}*/

//--------------------------------------------------------------------------------------------------
pub(crate) type ImageFrameResource<'imp> = FrameResource<'imp, Image, vk::ImageCreateInfo>;
pub(crate) type BufferFrameResource<'imp> = FrameResource<'imp, Buffer, vk::BufferCreateInfo>;

impl<'imp> ImageFrameResource<'imp> {
    pub fn dimensions(&self) -> (u32, u32, u32) {
        match self {
            FrameResource::Imported { resource } => resource.dimensions(),
            FrameResource::Transient {
                ref description, ..
            } => (
                description.extent.width,
                description.extent.height,
                description.extent.depth,
            ),
        }
    }

    pub fn format(&self) -> vk::Format {
        match self {
            FrameResource::Imported { resource } => resource.format(),
            FrameResource::Transient {
                ref description, ..
            } => description.format,
        }
    }
}

impl<'imp> BufferFrameResource<'imp> {
    pub fn size(&self) -> vk::DeviceSize {
        match self {
            FrameResource::Imported { resource } => resource.size(),
            FrameResource::Transient {
                ref description, ..
            } => description.size,
        }
    }
}


//--------------------------------------------------------------------------------------------------
struct Resources<'ctx> {
    /// Table of images used in this frame.
    pub(crate) images: Vec<ImageFrameResource<'ctx>>,
    /// Table of buffers used in this frame.
    pub(crate) buffers: Vec<BufferFrameResource<'ctx>>,
}

impl<'ctx> Resources<'ctx> {

    /// Gets the dimensions of the image (width, height, depth).
    pub fn get_image_dimensions(&self, img: ImageId) -> (u32, u32, u32) {
        self.images[img.0 as usize].dimensions()
    }

    /// Gets the dimensions of the image.
    pub fn get_image_format(&self, img: ImageId) -> vk::Format {
        self.images[img.0 as usize].format()
    }

    fn create_image(&mut self, name: impl Into<String>, desc: ImageDesc) -> ImageId {
        // get an index to generate a name for this resource.
        // It's not crucial that we get a unique one,
        // as the name of resources are here for informative purposes only.
        let naming_index = self.images.len();
        self.add_image_resource(name.into(), desc)
    }

    pub fn get_image_desc(&self, ) -> ImageDesc {

    }

    /// Adds a transient buffer resource.
    pub(crate) fn add_buffer_resource(&mut self, name: String, desc: BufferDesc) -> BufferId {
        self.buffers
            .push(BufferFrameResource::new_transient(name, desc));
        BufferId((self.buffers.len() - 1) as u32)
    }

    /// Adds a transient image resource.
    pub(crate) fn add_image_resource(&mut self, name: String, desc: ImageDesc) -> ImageId {
        self.images
            .push(ImageFrameResource::new_transient(name, desc));
        ImageId((self.images.len() - 1) as u32)
    }
}
