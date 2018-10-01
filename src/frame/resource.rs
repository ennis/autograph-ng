use super::*;
use sid_vec::{Id, IdVec};

//--------------------------------------------------------------------------------------------------

pub struct ImageTag;
/// Identifies an image in the frame resource table.
pub type ImageId = Id<ImageTag, u32>;

pub struct BufferTag;
/// Identifies a buffer in the frame resource table.
pub type BufferId = Id<BufferTag, u32>;

pub struct AttachmentTag;
pub type AttachmentIndex = Id<AttachmentTag, u32>;

/// A special type of resource reference that identifies an image resource used as an attachment
/// between subpasses.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AttachmentId {
    pub(crate) img: ImageId,
    pub(crate) renderpass: RenderPassId,
    pub(crate) index: AttachmentIndex,
}

//--------------------------------------------------------------------------------------------------
/// A resource (image or buffer) used in a frame.
pub enum FrameResource<'imp, T: Resource + 'imp> {
    Imported {
        resource: &'imp T,
    },
    Transient {
        name: String,
        create_info: <T as Resource>::CreateInfo,
        resource: Option<T>,
    },
}

impl<'imp, T: Resource> FrameResource<'imp, T> {
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

    pub fn new_transient(
        name: String,
        create_info: <T as Resource>::CreateInfo,
    ) -> FrameResource<'imp, T> {
        FrameResource::Transient {
            name,
            create_info,
            resource: None,
        }
    }

    pub fn new_imported(resource: &'imp T) -> FrameResource<'imp, T> {
        FrameResource::Imported { resource }
    }

    pub fn create_info(&self) -> &<T as Resource>::CreateInfo {
        match self {
            FrameResource::Transient {
                ref create_info, ..
            } => create_info,
            FrameResource::Imported { ref resource } => resource.create_info(),
        }
    }

    pub fn create_info_mut(&mut self) -> Option<&mut <T as Resource>::CreateInfo> {
        match self {
            FrameResource::Transient {
                ref mut create_info,
                ..
            } => Some(create_info),
            _ => None,
        }
    }
}

impl<'imp> ImageFrameResource<'imp> {
    pub(crate) fn get_initial_layout(&self) -> vk::ImageLayout {
        match self {
            FrameResource::Transient {
                ref create_info, ..
            } => create_info.initial_layout,
            FrameResource::Imported { resource } => resource.last_layout(),
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
pub(crate) type ImageFrameResource<'imp> = FrameResource<'imp, Image>;
pub(crate) type BufferFrameResource<'imp> = FrameResource<'imp, Buffer>;

//--------------------------------------------------------------------------------------------------
pub(crate) struct Resources<'ctx> {
    /// Table of images used in this frame.
    pub(crate) images: IdVec<ImageId, ImageFrameResource<'ctx>>,
    /// Table of buffers used in this frame.
    pub(crate) buffers: IdVec<BufferId, BufferFrameResource<'ctx>>,
}

impl<'ctx> Resources<'ctx> {
    ///
    pub(crate) fn new() -> Resources<'ctx> {
        Resources {
            images: IdVec::new(),
            buffers: IdVec::new(),
        }
    }

    /// Gets the dimensions of the image.
    pub(crate) fn get_image_create_info(&self, img: ImageId) -> &vk::ImageCreateInfo {
        self.images[img].create_info()
    }

    pub(crate) fn create_image(
        &mut self,
        name: impl Into<String>,
        create_info: vk::ImageCreateInfo,
    ) -> ImageId {
        self.images
            .push(ImageFrameResource::new_transient(name.into(), create_info))
    }

    /// Adds a transient buffer resource.
    pub(crate) fn create_buffer(
        &mut self,
        name: impl Into<String>,
        create_info: vk::BufferCreateInfo,
    ) -> BufferId {
        self.buffers
            .push(BufferFrameResource::new_transient(name.into(), create_info))
    }

    pub(crate) fn add_imported_image(&mut self, img: &'ctx Image) -> ImageId {
        self.images.push(ImageFrameResource::new_imported(img))
    }

    /// Adds a usage bit to an image resource.
    pub(crate) fn add_or_check_image_usage(&mut self, img: ImageId, usage: vk::ImageUsageFlags) {
        match &mut self.images[img] {
            FrameResource::Transient {
                ref mut create_info,
                ..
            } => {
                create_info.usage |= usage;
            }
            FrameResource::Imported { ref resource } => { } // TODO assert!(resource.usage().subset(usage)),
        }
    }
}
