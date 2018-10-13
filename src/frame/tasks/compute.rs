use super::*;

#[derive(Debug)]
pub(crate) struct ComputeTask {}

impl ComputeTask {
    pub fn new() -> ComputeTask {
        ComputeTask {}
    }
}

//--------------------------------------------------------------------------------------------------
pub struct ComputeTaskBuilder<'a, 'ctx: 'a> {
    graph: &'a mut FrameGraph,
    resources: &'a mut Resources<'ctx>,
    task: TaskId,
    compute_task: ComputeTask,
}

impl<'a, 'ctx: 'a> ComputeTaskBuilder<'a, 'ctx> {
    pub(super) fn new(
        name: impl Into<String>,
        graph: &'a mut FrameGraph,
        resources: &'a mut Resources<'ctx>,
    ) -> ComputeTaskBuilder<'a, 'ctx> {
        let task = graph.create_task_on_queue(name, 0, TaskDetails::Other);
        ComputeTaskBuilder {
            graph,
            resources,
            task,
            compute_task: ComputeTask::new(),
        }
    }

    pub(super) fn finish(mut self) -> TaskId {
        self.graph.0.node_weight_mut(self.task).unwrap().details =
            TaskDetails::Compute(self.compute_task);
        self.task
    }
}
