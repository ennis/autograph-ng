use frame::resource::{BufferId, ImageId};

use ash::vk;

//--------------------------------------------------------------------------------------------------

/// One side of an image memory barrier.
#[derive(Copy, Clone, Debug)]
pub struct ImageMemoryBarrierHalf {
    pub stage_mask: vk::PipelineStageFlags,
    pub access_mask: vk::AccessFlags,
    pub layout: vk::ImageLayout,
}

/// One side of a buffer memory barrier.
#[derive(Copy, Clone, Debug)]
pub struct BufferMemoryBarrierHalf {
    pub stage_mask: vk::PipelineStageFlags,
    pub access_mask: vk::AccessFlags,
}

//--------------------------------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct ImageMemoryBarrier {
    pub id: ImageId,
    pub src_access_mask: vk::AccessFlags,
    pub dst_access_mask: vk::AccessFlags,
    pub old_layout: vk::ImageLayout,
    pub new_layout: vk::ImageLayout,
}

#[derive(Clone, Debug)]
pub struct BufferMemoryBarrier {
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
    Image(ImageMemoryBarrier),
    /// Buffer dependency. Analogous to  `VkBufferMemoryBarrier`.
    Buffer(BufferMemoryBarrier),
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

    pub fn with_image_memory_barrier(
        image: ImageId,
        source: ImageMemoryBarrierHalf,
        destination: ImageMemoryBarrierHalf,
    ) -> Dependency {
        Dependency {
            src_stage_mask: source.stage_mask,
            dst_stage_mask: destination.stage_mask,
            latency: 0,
            barrier: BarrierDetail::Image(ImageBarrier {
                src_access_mask: source.access_mask,
                dst_access_mask: destination.access_mask,
                id: image,
                old_layout: destination.layout,
                new_layout: destination.layout,
            }),
        }
    }

    pub fn with_buffer_memory_barrier(
        buffer: BufferId,
        source: BufferMemoryBarrierHalf,
        destination: BufferMemoryBarrierHalf,
    ) -> Dependency {
        Dependency {
            src_stage_mask: source.stage_mask,
            dst_stage_mask: destination.stage_mask,
            latency: 0,
            barrier: BarrierDetail::Buffer(BufferMemoryBarrier {
                src_access_mask: source.access_mask,
                dst_access_mask: destination.access_mask,
                id: buffer,
            }),
        }
    }

    pub fn sequence() -> Dependency {
        unimplemented!()
    }
}

pub fn is_write_access(access_flags: vk::AccessFlags) -> bool {
    access_flags.intersects(
        vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT
            | vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT
            | vk::ACCESS_HOST_WRITE_BIT
            | vk::ACCESS_MEMORY_WRITE_BIT
            | vk::ACCESS_SHADER_WRITE_BIT
            | vk::ACCESS_TRANSFER_WRITE_BIT,
    )
}
