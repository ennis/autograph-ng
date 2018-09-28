use super::resource::{BufferId, ImageId};
use super::*;

//--------------------------------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub(crate) struct ImageBarrier {
    pub(crate) id: ImageId,
    pub(crate) src_access_mask: vk::AccessFlags,
    pub(crate) dst_access_mask: vk::AccessFlags,
    pub(crate) old_layout: vk::ImageLayout,
    pub(crate) new_layout: vk::ImageLayout,
}

#[derive(Clone, Debug)]
pub(crate) struct BufferBarrier {
    pub(crate) id: BufferId,
    pub(crate) src_access_mask: vk::AccessFlags,
    pub(crate) dst_access_mask: vk::AccessFlags,
}

#[derive(Clone, Debug)]
pub(crate) struct SubpassBarrier {
    /// Must correspond to an attachment of the subpass.
    pub(crate) id: ImageId,
    pub(crate) src_access_mask: vk::AccessFlags,
    pub(crate) dst_access_mask: vk::AccessFlags,
    pub(crate) old_layout: vk::ImageLayout,
    pub(crate) new_layout: vk::ImageLayout,
}

/// Details of a dependency that is specific to the usage of the resource, and its
/// type.
#[derive(Clone, Debug)]
pub(crate) enum BarrierDetail {
    /// Image dependency. Analogous to `VkImageMemoryBarrier`.
    Image(ImageBarrier),
    /// Buffer dependency. Analogous to  `VkBufferMemoryBarrier`.
    Buffer(BufferBarrier),
    /// Dependency between subpasses.
    Subpass(SubpassBarrier),
    /// Represents a sequencing constraint between tasks.
    /// Not associated to a particular resource.
    Sequence,
}

/// Represents a dependency between tasks in the frame graph.
#[derive(Debug)]
pub(crate) struct Dependency {
    /// The pipeline stage must have completed on the dependency.
    /// By default, this is BOTTOM_OF_PIPE.
    pub(crate) src_stage_mask: vk::PipelineStageFlags,
    /// The pipeline stage of this task (destination) that is waiting on the dependency.
    /// By default, this is TOP_OF_PIPE.
    pub(crate) dst_stage_mask: vk::PipelineStageFlags,
    /// Estimated latency of the dependency (time for the resource to be usable by target once source is submitted).
    /// 0 for dummy nodes.
    pub(crate) latency: u32,
    /// Details of the dependency specific to the usage and the type of resource.
    pub(crate) barrier: BarrierDetail,
}

impl Dependency {
    /// Returns the image ID associated to the dependency, or None if the dependency has no resource associated to it
    /// or if the associated resource is not an image.
    pub(crate) fn get_image_id(&self) -> Option<ImageId> {
        match self.barrier {
            BarrierDetail::Image(ImageBarrier { id, .. }) => Some(id),
            _ => None,
        }
    }

    /// Returns the buffer ID associated to the dependency, or None if the dependency has no resource associated to it
    /// or if the associated resource is not a buffer.
    pub(crate) fn get_buffer_id(&self) -> Option<BufferId> {
        match self.barrier {
            BarrierDetail::Buffer(BufferBarrier { id, .. }) => Some(id),
            _ => None,
        }
    }

    pub(crate) fn as_image_barrier_mut(&mut self) -> Option<&mut ImageBarrier> {
        match self.barrier {
            BarrierDetail::Image(ref mut barrier) => Some(barrier),
            _ => None,
        }
    }

    pub(crate) fn as_buffer_barrier_mut(&mut self) -> Option<&mut BufferBarrier> {
        match self.barrier {
            BarrierDetail::Buffer(ref mut barrier) => Some(barrier),
            _ => None,
        }
    }
}
