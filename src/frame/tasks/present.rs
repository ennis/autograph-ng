use frame::dependency::{BarrierDetail, Dependency, ImageBarrier};
use frame::graph::TaskId;
use frame::resource::*;
use frame::tasks::{BufferRef, DummyTask, ImageRef, Task, TaskKind};
use frame::Frame;

use ash::vk;

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub(crate) struct PresentTask {
    images: Vec<ImageId>,
}

impl PresentTask {
    fn new() -> PresentTask {
        PresentTask { images: Vec::new() }
    }
}

impl Task for PresentTask {
    fn name(&self) -> &str {
        "present"
    }

    fn kind(&self) -> TaskKind {
        TaskKind::Present
    }
}

//--------------------------------------------------------------------------------------------------
pub struct PresentTaskBuilder<'id: 'a, 'a, 'imp: 'a> {
    frame: &'a mut Frame<'id, 'imp>,
    task: TaskId,
    present_task: PresentTask,
}

impl<'id: 'a, 'a, 'imp: 'a> PresentTaskBuilder<'id, 'a, 'imp> {
    pub fn new(frame: &'a mut Frame<'id, 'imp>) -> PresentTaskBuilder<'id, 'a, 'imp> {
        let task = frame.create_task(DummyTask);
        PresentTaskBuilder {
            frame,
            task,
            present_task: PresentTask::new(),
        }
    }

    pub fn present(&mut self, img: &ImageRef) {
        self.frame.add_dependency(
            img.task,
            self.task,
            Dependency {
                src_stage_mask: vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                dst_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                barrier: BarrierDetail::Image(ImageBarrier {
                    id: img.id(),
                    src_access_mask: vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                    dst_access_mask: vk::ACCESS_MEMORY_READ_BIT,
                    old_layout: vk::ImageLayout::Undefined,
                    new_layout: vk::ImageLayout::PresentSrcKhr,
                }),
                latency: img.latency,
            },
        );
        self.present_task.images.push(img.id());
    }

    pub fn finish(mut self) -> TaskId {
        self.frame.set_task(self.task, self.present_task);
        self.task
    }
}
