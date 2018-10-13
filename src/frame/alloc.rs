//! resource allocation

use super::*;

use ash::vk;
use sid_vec::{Id, IdVec};

struct PhysicalImageTag;
type PhysicalImageId = Id<PhysicalImageTag>;

struct PhysicalBufferTag;
type PhysicalBufferId = Id<PhysicalBufferTag>;

struct PhysicalImageAlloc {
    image: VkHandle<vk::Image>,
    image_type: vk::ImageType,
    dimensions: vk::Extent3D,
    format: vk::Format,
    /// In which memory type the image was allocated.
    memory_type_index: u32,
}

struct PhysicalBufferAlloc {
    buffer: VkHandle<vk::Buffer>,
    size: vk::DeviceSize,
}

struct Allocations {
    physical_images: IdVec<PhysicalImageId, PhysicalImageAlloc>,
    physical_buffers: IdVec<PhysicalBufferId, PhysicalBufferAlloc>,
    /// Mapping from virtual image to physical image (index in physical_images).
    images: IdVec<ImageId, u32>,
    /// Mapping from virtual buffer to physical buffer.
    buffers: IdVec<BufferId, u32>,
}

impl Allocations {
    pub fn find_compatible_image(&self) -> Option<PhysicalImageId> {
        for (i, img) in self.physical_images.iter().enumerate() {
            let id = PhysicalImageId::from_index(i);
        }
        None
    }
}

fn allocate_physical_resources(
    g: &FrameGraphInner,
    resources: &Resources,
    allocator: &Allocator,
) -> Allocations {
    // rules of allocation:
    // - the allocator will never add a barrier to wait for a resource to be free if the
    //   specified dependencies are not enough to prove it
    // - a resource can be aliased with another if:
    //   1. it is provably free at that point in execution:
    //      i.e. there is a dependency path between the first task using this resource
    //           and all nodes that used this resource for the last time.
    //   2. it is compatible
    //      i.e. same size and memory properties

}
