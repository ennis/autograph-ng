use std::cell::Cell;
use std::marker::PhantomData;

use frame::graph::TaskId;
use frame::resource::{BufferId, ImageId};
use frame::{Frame, LifetimeId};

use ash::vk;

pub mod present;
//mod compute;
//mod graphics;
//mod transfer;

use self::present::PresentTask;
//use self::graphics::GraphicsTask;
//use super::compute::ComputeTask;
//use super::transfer::TransferTask;

#[derive(Debug)]
pub enum TaskKind {
    Graphics,
    Compute,
    Transfer,
    Present,
    RayTracing,
    Other,
}

/// Represents an operation in the frame graph.
pub trait Task {
    fn name(&self) -> &str;
    fn kind(&self) -> TaskKind;
}

#[derive(Debug)]
pub struct RayTracingTask {}

#[derive(Debug)]
pub struct DummyTask;

impl Task for DummyTask {
    fn name(&self) -> &str {
        "dummy"
    }

    fn kind(&self) -> TaskKind {
        TaskKind::Other
    }
}

impl DummyTask {
    pub fn new() -> DummyTask {
        DummyTask
    }
}

//--------------------------------------------------------------------------------------------------
pub trait TaskOutput<'id> {
    fn task(&self) -> TaskId;
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
pub struct BufferRef<'id> {
    _lifetime: LifetimeId<'id>,
    buffer: BufferId,
    task: TaskId,
    src_stage_mask: vk::PipelineStageFlags,
    src_access: vk::AccessFlags,
    read_flag: Cell<bool>,
    write_flag: Cell<bool>,
    latency: u32,
}

impl<'id> TaskOutput<'id> for BufferRef<'id> {
    fn task(&self) -> TaskId {
        self.task
    }

    fn latency(&self) -> u32 {
        self.latency
    }

    fn src_stage_mask(&self) -> vk::PipelineStageFlags {
        self.src_stage_mask
    }

    fn src_access(&self) -> vk::AccessFlags {
        self.src_access
    }

    fn read_flag(&self) -> &Cell<bool> {
        &self.read_flag
    }

    fn write_flag(&self) -> &Cell<bool> {
        &self.write_flag
    }
}

//--------------------------------------------------------------------------------------------------
pub struct ImageRef<'id> {
    _lifetime: LifetimeId<'id>,
    image: ImageId,
    task: TaskId,
    src_stage_mask: vk::PipelineStageFlags,
    src_access: vk::AccessFlags,
    layout: vk::ImageLayout,
    read_flag: Cell<bool>,
    write_flag: Cell<bool>,
    latency: u32,
}

impl<'id> ImageRef<'id> {
    pub fn new(
        image: ImageId,
        task: TaskId,
        src_stage_mask: vk::PipelineStageFlags,
        src_access: vk::AccessFlags,
        layout: vk::ImageLayout,
        latency: u32,
    ) -> ImageRef<'id> {
        ImageRef {
            _lifetime: PhantomData,
            image,
            task,
            src_stage_mask,
            src_access,
            layout,
            read_flag: Cell::new(false),
            write_flag: Cell::new(false),
            latency: 0,
        }
    }

    pub fn id(&self) -> ImageId {
        self.image
    }

    /*pub fn dimensions(&self) -> ImageDimensions {
        self.frame.image_resource(self.image).dimensions()
    }*/
}

impl<'id> TaskOutput<'id> for ImageRef<'id> {
    fn task(&self) -> TaskId {
        self.task
    }

    fn latency(&self) -> u32 {
        self.latency
    }

    fn src_stage_mask(&self) -> vk::PipelineStageFlags {
        self.src_stage_mask
    }

    fn src_access(&self) -> vk::AccessFlags {
        self.src_access
    }

    fn read_flag(&self) -> &Cell<bool> {
        &self.read_flag
    }

    fn write_flag(&self) -> &Cell<bool> {
        &self.write_flag
    }
}
