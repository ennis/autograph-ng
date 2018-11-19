use std::ptr;

use crate::frame::dependency::{
    BarrierDetail, Dependency, ImageMemoryBarrier, ImageMemoryBarrierHalf,
};
use crate::frame::resource::*;
use crate::frame::tasks::{
    BufferRef, DummyTask, ImageRef, ScheduleContext, Pass, TaskKind, TaskOperationType,
};
use crate::frame::Frame;
use crate::frame::PassId;

use ash::vk;

/*
//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct PresentTask {
    images: Vec<ImageId>,
}

impl PresentTask {
    fn new() -> PresentTask {
        PresentTask { images: Vec::new() }
    }
}

impl Pass for PresentTask {
    #[inline]
    fn name(&self) -> &str {
        "present"
    }

    #[inline]
    fn kind(&self) -> TaskKind {
        TaskKind::Present
    }

    #[inline]
    fn schedule<'sctx>(&self, sctx: &ScheduleContext<'sctx>) {
        let queue = sctx.queue().expect("unexpected context");
        let wait_semaphores = sctx.wait_semaphores();

        let vkd = sctx.device.pointers();
        let vk_khr_swapchain = &sctx.device.extension_pointers().vk_khr_swapchain;

        let mut swapchains = Vec::new();
        let mut swapchain_indices = Vec::new();
        for &id in self.images.iter() {
            let image = sctx.get_image(id);
            swapchains.push(
                image
                    .swapchain()
                    .expect("attempted to present a non-swapchain image"),
            );
            swapchain_indices.push(image.swapchain_index().unwrap());
        }

        let mut results = vec![vk::Result::Success; swapchains.len()];

        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PresentInfoKhr,
            p_next: ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            swapchain_count: swapchains.len() as u32,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: swapchain_indices.as_ptr(),
            p_results: results.as_mut_ptr(),
        };

        unsafe {
            vk_khr_swapchain
                .queue_present_khr(queue, &present_info)
                .unwrap();
        }
    }
}

//--------------------------------------------------------------------------------------------------
pub struct PresentTaskBuilder<'a, 'id: 'a> {
    frame: &'a mut Frame<'id>,
    task: PassId,
    present_task: PresentTask,
}

impl<'a, 'id: 'a> PresentTaskBuilder<'a, 'id> {
    pub fn new(frame: &'a mut Frame<'id>) -> PresentTaskBuilder<'a, 'id> {
        let task = frame.create_task(DummyTask);
        PresentTaskBuilder {
            frame,
            task,
            present_task: PresentTask::new(),
        }
    }

    pub fn present(&mut self, image: &ImageRef<'id>) {
        self.frame.add_image_dependency(
            self.task,
            image,
            vk::ImageUsageFlags::empty(),
            ImageMemoryBarrierHalf {
                stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                access_mask: vk::ACCESS_MEMORY_READ_BIT,
                layout: vk::ImageLayout::PresentSrcKhr,
            },
        );
        self.present_task.images.push(image.id());
    }

    pub fn finish(mut self) -> PassId {
        self.frame.set_task(self.task, self.present_task);
        self.task
    }
}
*/