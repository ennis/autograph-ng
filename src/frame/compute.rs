use super::*;

#[derive(Debug)]
pub(crate) struct ComputeTask {}

impl ComputeTask {
    pub fn new() -> ComputeTask {
        ComputeTask {}
    }
}

//--------------------------------------------------------------------------------------------------
pub struct ComputeTaskBuilder<'frame, 'ctx: 'frame> {
    frame: &'frame mut Frame<'ctx>,
    task: TaskId,
    compute_task: ComputeTask,
}

impl<'frame, 'ctx: 'frame> ComputeTaskBuilder<'frame, 'ctx> {
    pub(super) fn new(
        frame: &'frame mut Frame<'ctx>,
        name: impl Into<String>,
    ) -> ComputeTaskBuilder<'frame, 'ctx> {
        let task = frame.create_task_on_queue(name, 0, TaskDetails::Other);
        ComputeTaskBuilder {
            frame,
            task,
            compute_task: ComputeTask::new(),
        }
    }

    pub(super) fn finish(mut self) -> TaskId {
        self.frame
            .graph
            .node_weight_mut(self.task)
            .unwrap()
            .details = TaskDetails::Compute(self.compute_task);
        self.task
    }
}
