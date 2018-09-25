use super::resource::{BufferId, ImageId};
use super::*;

//--------------------------------------------------------------------------------------------------

/// Details of a dependency that is specific to the usage of the resource, and its
/// type.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DependencyResource {
    /// Image dependency: either a sampled image or a storage image.
    /// This produces the image barrier.
    Image(ImageId),
    Buffer(BufferId),
    /// Represents a sequencing constraint between tasks.
    /// Not associated to a particular resource.
    Sequence,
}

impl From<ImageId> for DependencyResource {
    fn from(id: ImageId) -> Self {
        DependencyResource::Image(id)
    }
}

impl From<BufferId> for DependencyResource {
    fn from(id: BufferId) -> Self {
        DependencyResource::Buffer(id)
    }
}

/// Represents a dependency between tasks in the frame graph.
#[derive(Debug)]
pub(crate) struct Dependency {
    /// How this resource is accessed by the dependent task.
    /// See vulkan docs for all possible flags.
    pub(crate) access_bits: vk::AccessFlags,
    /// What pipeline stage must have completed on the dependency.
    /// By default, this is BOTTOM_OF_PIPE.
    pub(crate) src_stage_mask: vk::PipelineStageFlags,
    /// What pipeline stage of this task (destination) is waiting on the dependency.
    /// By default, this is TOP_OF_PIPE.
    pub(crate) dst_stage_mask: vk::PipelineStageFlags,
    /// Estimated latency of the dependency (time for the resource to be usable by target once source is submitted).
    /// 0 for dummy nodes.
    pub(crate) latency: u32,
    /// Details of the dependency specific to the usage and the type of resource.
    /// TODO replace this with 'barrier detail'
    pub(crate) resource: DependencyResource,
}

impl Dependency {
    pub(crate) fn get_image_id(&self) -> Option<ImageId> {
        match self.resource {
            DependencyResource::Image(id) => Some(id),
            _ => None,
        }
    }

    pub(crate) fn get_buffer_id(&self) -> Option<BufferId> {
        match self.resource {
            DependencyResource::Buffer(id) => Some(id),
            _ => None,
        }
    }
}
