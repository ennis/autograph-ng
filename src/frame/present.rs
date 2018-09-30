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
pub struct PresentTaskBuilder<'a, 'ctx: 'a> {
    graph: &'a mut FrameGraph,
    resources: &'a mut Resources<'ctx>,
    task: TaskId,
    present_task: PresentTask,
}

impl<'a, 'ctx: 'a> PresentTaskBuilder<'a, 'ctx> {
    pub(super) fn new(
        name: impl Into<String>,
        graph: &'a mut FrameGraph,
        resources: &'a mut Resources<'ctx>,
    ) -> PresentTaskBuilder<'a, 'ctx> {
        let task = graph.create_task_on_queue(name, 2, TaskDetails::Other);
        PresentTaskBuilder {
            graph,
            resources,
            task,
            present_task: PresentTask::new(),
        }
    }

    pub fn present(&mut self, img: &ImageRef) {
        self.graph.add_dependency(
            img.task,
            self.task,
            Dependency {
                src_stage_mask: vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                dst_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                barrier: BarrierDetail::Image(ImageBarrier {
                    id: img.id,
                    src_access_mask: vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                    dst_access_mask: vk::ACCESS_MEMORY_READ_BIT,
                    old_layout: vk::ImageLayout::Undefined,
                    new_layout: vk::ImageLayout::PresentSrcKhr,
                }),
                latency: img.latency,
            },
        );
        self.present_task.images.push(img.id);
    }

    pub(super) fn finish(mut self) -> TaskId {
        self.graph.0.node_weight_mut(self.task).unwrap().details =
            TaskDetails::Present(self.present_task);
        self.task
    }
}
