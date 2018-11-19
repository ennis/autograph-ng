use std::cell::Cell;
use std::marker::PhantomData;
use std::ops::Deref;

use ash::vk;

use crate::device::{Device, VkDevice1};
use crate::frame::dependency::{BufferMemoryBarrierHalf, ImageMemoryBarrierHalf};
use crate::frame::resource::{BufferId, BufferResource, ImageId, ImageResource};
use crate::frame::PassId;
use crate::frame::{Frame, LifetimeId};

/// Represents an operation in the frame graph.
pub trait Pass {
    fn name(&self) -> &str;

    fn preferred_queue(&self) -> Option<(u32, vk::Queue)> {
        None
    }
}

/*pub fn select_queue_for_task(task: &Task, device: &Device) -> (u32, vk::Queue) {
    if let Some(queue) = task.preferred_queue() {
        queue
    } else {
        match task.kind() {
            TaskKind::Graphics => device.default_graphics_queue(),
            TaskKind::Compute => device.default_compute_queue(),
            TaskKind::Transfer => device.default_transfer_queue(),
            TaskKind::Present => device.default_present_queue(),
            TaskKind::RayTracing => device.default_graphics_queue(),
            TaskKind::Other => device.default_graphics_queue(),
        }
    }
}*/

#[derive(Debug)]
pub struct DummyPass;

impl Pass for DummyPass {
    #[inline]
    fn name(&self) -> &str {
        "dummy"
    }
}

impl DummyPass {
    pub fn new() -> DummyPass {
        DummyPass
    }
}

//--------------------------------------------------------------------------------------------------
pub trait PassOutput {
    fn pass(&self) -> Option<PassId>;
    fn latency(&self) -> u32;
    fn src_stage_mask(&self) -> vk::PipelineStageFlags;
    fn src_access(&self) -> vk::AccessFlags;
    fn read_flag(&self) -> &Cell<bool>;
    fn write_flag(&self) -> &Cell<bool>;

    fn set_write_flag(&self) -> Result<(), ()> {
        let read = self.read_flag();
        let written = self.write_flag();
        if read.get() {
            return Err(());
        }
        if written.get() {
            return Err(());
        }
        written.set(true);
        Ok(())
    }

    fn set_read_flag(&self) -> Result<(), ()> {
        let read = self.read_flag();
        let written = self.write_flag();
        if written.get() {
            return Err(());
        }
        read.set(true);
        Ok(())
    }
}

//--------------------------------------------------------------------------------------------------
pub struct BufferRef {
    buffer: BufferId,
    pass: Option<PassId>,
    src_barrier: BufferMemoryBarrierHalf,
    read_flag: Cell<bool>,
    write_flag: Cell<bool>,
    latency: u32,
}

impl PassOutput for BufferRef {
    #[inline]
    fn pass(&self) -> Option<PassId> {
        self.pass
    }

    #[inline]
    fn latency(&self) -> u32 {
        self.latency
    }

    #[inline]
    fn src_stage_mask(&self) -> vk::PipelineStageFlags {
        self.src_barrier.stage_mask
    }

    #[inline]
    fn src_access(&self) -> vk::AccessFlags {
        self.src_barrier.access_mask
    }

    #[inline]
    fn read_flag(&self) -> &Cell<bool> {
        &self.read_flag
    }

    #[inline]
    fn write_flag(&self) -> &Cell<bool> {
        &self.write_flag
    }
}

//--------------------------------------------------------------------------------------------------
pub struct ImageRef {
    image: ImageId,
    pass: Option<PassId>,
    src_barrier: ImageMemoryBarrierHalf,
    read_flag: Cell<bool>,
    write_flag: Cell<bool>,
    latency: u32,
}

impl ImageRef {
    pub fn new(
        image: ImageId,
        pass: Option<PassId>,
        src_barrier: ImageMemoryBarrierHalf,
    ) -> ImageRef
    {
        ImageRef {
            image,
            pass,
            src_barrier,
            read_flag: Cell::new(false),
            write_flag: Cell::new(false),
            latency: 0,
        }
    }

    pub fn id(&self) -> ImageId {
        self.image
    }

    pub fn src_barrier(&self) -> &ImageMemoryBarrierHalf {
        &self.src_barrier
    }

}

impl PassOutput for ImageRef
{
    #[inline]
    fn pass(&self) -> Option<PassId> {
        self.pass
    }

    #[inline]
    fn latency(&self) -> u32 {
        self.latency
    }

    #[inline]
    fn src_stage_mask(&self) -> vk::PipelineStageFlags {
        self.src_barrier.stage_mask
    }

    #[inline]
    fn src_access(&self) -> vk::AccessFlags {
        self.src_barrier.access_mask
    }

    #[inline]
    fn read_flag(&self) -> &Cell<bool> {
        &self.read_flag
    }

    #[inline]
    fn write_flag(&self) -> &Cell<bool> {
        &self.write_flag
    }
}
