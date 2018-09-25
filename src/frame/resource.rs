use super::*;

//--------------------------------------------------------------------------------------------------
/// Identifies an image in the frame resource table.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ImageId(pub(crate) u32);

/// Identifies a buffer in the frame resource table.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BufferId(pub(crate) u32);

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
pub(crate) struct ImageDesc {
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
}

//--------------------------------------------------------------------------------------------------
pub(crate) type ImageFrameResource<'imp> = FrameResource<'imp, Image, ImageDesc>;
pub(crate) type BufferFrameResource<'imp> = FrameResource<'imp, Buffer, BufferDesc>;

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
