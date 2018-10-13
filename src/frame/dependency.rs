use frame::resource::{BufferId, ImageId};

use ash::vk;

//--------------------------------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct ImageBarrier {
    pub id: ImageId,
    pub src_access_mask: vk::AccessFlags,
    pub dst_access_mask: vk::AccessFlags,
    pub old_layout: vk::ImageLayout,
    pub new_layout: vk::ImageLayout,
}

#[derive(Clone, Debug)]
pub struct BufferBarrier {
    pub id: BufferId,
    pub src_access_mask: vk::AccessFlags,
    pub dst_access_mask: vk::AccessFlags,
}

#[derive(Clone, Debug)]
pub struct SubpassBarrier {
    pub id: ImageId,
    pub src_access_mask: vk::AccessFlags,
    pub dst_access_mask: vk::AccessFlags,
    pub old_layout: vk::ImageLayout,
    pub new_layout: vk::ImageLayout,
}

/// Details of a dependency that is specific to the usage of the resource, and its
/// type.
#[derive(Clone, Debug)]
pub enum BarrierDetail {
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
pub struct Dependency {
    /// The pipeline stage must have completed on the dependency.
    /// By default, this is BOTTOM_OF_PIPE.
    pub src_stage_mask: vk::PipelineStageFlags,
    /// The pipeline stage of this task (destination) that is waiting on the dependency.
    /// By default, this is TOP_OF_PIPE.
    pub dst_stage_mask: vk::PipelineStageFlags,
    /// Estimated latency of the dependency (time for the resource to be usable by target once source is submitted).
    /// 0 for dummy nodes.
    pub latency: u32,
    /// Details of the dependency specific to the usage and the type of resource.
    pub barrier: BarrierDetail,
}

impl Dependency {
    /// The pipeline stage must have completed on the dependency.
    /// By default, this is BOTTOM_OF_PIPE.
    pub fn src_stage_mask(&self) -> vk::PipelineStageFlags {
        self.src_stage_mask
    }

    /// The pipeline stage of this task (destination) that is waiting on the dependency.
    /// By default, this is TOP_OF_PIPE.
    pub fn dst_stage_mask(&self) -> vk::PipelineStageFlags {
        self.dst_stage_mask
    }

    /// Estimated latency of the dependency (time for the resource to be usable by target once source is submitted).
    /// 0 for dummy nodes.
    pub fn latency(&self) -> u32 {
        self.latency
    }

    /// Details of the dependency specific to the usage and the type of resource.
    pub fn barrier(&self) -> &BarrierDetail {
        &self.barrier
    }

    /// Details of the dependency specific to the usage and the type of resource.
    pub fn barrier_mut(&mut self) -> &mut BarrierDetail {
        &mut self.barrier
    }

    /// Returns the image ID associated to the dependency, or None if the dependency has no resource associated to it
    /// or if the associated resource is not an image.
    pub fn get_image_id(&self) -> Option<ImageId> {
        match self.barrier {
            BarrierDetail::Image(ImageBarrier { id, .. }) => Some(id),
            _ => None,
        }
    }

    /// Returns the buffer ID associated to the dependency, or None if the dependency has no resource associated to it
    /// or if the associated resource is not a buffer.
    pub fn get_buffer_id(&self) -> Option<BufferId> {
        match self.barrier {
            BarrierDetail::Buffer(BufferBarrier { id, .. }) => Some(id),
            _ => None,
        }
    }

    pub fn as_image_barrier_mut(&mut self) -> Option<&mut ImageBarrier> {
        match self.barrier {
            BarrierDetail::Image(ref mut barrier) => Some(barrier),
            _ => None,
        }
    }

    pub fn as_buffer_barrier_mut(&mut self) -> Option<&mut BufferBarrier> {
        match self.barrier {
            BarrierDetail::Buffer(ref mut barrier) => Some(barrier),
            _ => None,
        }
    }
}
