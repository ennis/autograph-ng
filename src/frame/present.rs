use super::*;

#[derive(Debug)]
pub(crate) struct PresentTask {
    images: Vec<ImageId>,
}

impl PresentTask {
    fn new() -> PresentTask {
        PresentTask { images: Vec::new() }
    }
}

//--------------------------------------------------------------------------------------------------
pub struct PresentTaskBuilder<'frame, 'ctx: 'frame> {
    frame: &'frame mut Frame<'ctx>,
    task: TaskId,
    present_task: PresentTask,
}

impl<'frame, 'ctx: 'frame> PresentTaskBuilder<'frame, 'ctx> {
    pub(super) fn new(
        frame: &'frame mut Frame<'ctx>,
        name: impl Into<String>,
    ) -> PresentTaskBuilder<'frame, 'ctx> {
        let task = frame.create_task_on_queue(name, 2, TaskDetails::Other);
        PresentTaskBuilder {
            frame,
            task,
            present_task: PresentTask::new(),
        }
    }

    pub fn present(&mut self, img: &ImageRef) {
        self.frame
            .add_generic_read_dependency(img.task, self.task, img.id);
        self.present_task.images.push(img.id);
    }

    pub(super) fn finish(mut self) -> TaskId {
        self.frame.graph.node_weight_mut(self.task).unwrap().details =
            TaskDetails::Present(self.present_task);
        self.task
    }
}
