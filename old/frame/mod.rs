//! Frame graphs
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::fs::File;
use std::io::{stdout, Write};
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::ptr;
use std::sync::Arc;

use ash::vk;
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    visit::EdgeRef,
    Directed, Direction, Graph,
};
use sid_vec::{Id, IdVec};

mod dependency;
mod dump;
mod graphviz;
mod resource;
mod sched;
pub mod pass;

use crate::device::{Device, FrameSynchronizedObject};
use crate::image::{Image, ImageDescription, ImageProxy};
use crate::swapchain::Swapchain;

pub use self::dependency::*;
use self::resource::*;
pub use self::pass::*;


//--------------------------------------------------------------------------------------------------
type LifetimeId<'id> = PhantomData<Cell<&'id mut ()>>;

//--------------------------------------------------------------------------------------------------
// Frame graph
type PassId = NodeIndex<u32>;
type DependencyId = EdgeIndex<u32>;

/// The frame graph type.
type FrameGraph = Graph<Box<Pass>, Dependency, Directed, u32>;

pub struct PassBuilder<'a>
{
    frame: &'a mut Frame,
    pass_id: PassId,
}

impl<'a> PassBuilder<'a>
{
    fn add_image_dependency(
        &mut self,
        image: &ImageRef,
        dst_barrier: ImageMemoryBarrierHalf,)
    {
        if let Some(src_task) = image.task() {
            let dependency = Dependency::with_image_memory_barrier(
                image.id(),
                *image.src_barrier(),
                dst_barrier,
            );
            self.frame.graph.add_edge(src_task, task, dependency)
        }
    }

    ///
    /// TODO DOCUMENT
    pub fn image_read(
        &mut self,
        image: &ImageRef,
        usage_flags: vk::ImageUsageFlags,
        dst_barrier: ImageMemoryBarrierHalf,
    )
    {
        assert!(!is_write_access(dst_barrier.access_mask));
        image.set_read_flag().expect("concurrent read/write conflict");
        self.frame.image_resource_mut(image.id()).set_usage(usage_flags);
        self.add_image_dependency(image, dst_barrier);
    }

    ///
    /// TODO DOCUMENT
    pub fn image_write(
        &mut self,
        image: &ImageRef,
        usage_flags: vk::ImageUsageFlags,
        dst_barrier: ImageMemoryBarrierHalf,) -> ImageRef
    {
        assert!(is_write_access(dst_barrier.access_mask));
        image.set_write_flag().expect("concurrent read/write conflict");
        self.frame.image_resource_mut(image.id()).set_usage(usage_flags);
        self.add_image_dependency(image, dst_barrier);
        ImageRef {
            pass: Some(self.pass_id),

        }
    }
}



//--------------------------------------------------------------------------------------------------
/// A frame: manages transient resources within a frame.
/// 'id is an invariant lifetime that should be used to tag resource references (ImageRefs and BufferRefs)
pub struct Frame {
    device: Arc<Device>,
    graph: FrameGraph,
    images: IdVec<ImageId, Box<ImageResource>>,
    buffers: IdVec<BufferId, Box<BufferResource>>,
}

//--------------------------------------------------------------------------------------------------
// PUBLIC API
impl Frame {
    /// Imports a persistent image for use in the frame graph.
    /// Borrows the input image.
    pub fn import_image<I, IP, ID>(&mut self, image: &I) -> ImageRef
    where
        IP: ImageProxy + 'static,
        ID: ImageDescription,
        I: FrameSynchronizedObject<Proxy = IP> + Deref<Target = ID>,
    {
        let current_frame = self.device.current_frame();
        let task = self.create_task(DummyTask);

        let imported_image = ImportedImageResource::new(image, self.device.current_frame());
        let initial_layout = imported_image.initial_layout();
        let image = self.images.push(Box::new(imported_image));

        ImageRef::new(
            image,
            Some(task),
            ImageMemoryBarrierHalf {
                stage_mask: vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
                access_mask: vk::ACCESS_MEMORY_WRITE_BIT,
                layout: initial_layout,
            },
        )
    }

    pub fn import_swapchain_image(&mut self, swapchain: &Arc<Swapchain>) -> ImageRef {
        let current_frame = self.device.current_frame();
        let task = self.create_task(DummyTask);
        let swapchain_image = SwapchainImageResource::new(swapchain, self.device.current_frame());
        let initial_layout = swapchain_image.initial_layout();
        let image = self.images.push(Box::new(swapchain_image));

        ImageRef::new(
            image,
            Some(task),
            ImageMemoryBarrierHalf {
                stage_mask: vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
                access_mask: vk::ACCESS_MEMORY_WRITE_BIT,
                layout: initial_layout,
            },
        )
    }

    pub fn submit(mut self) {
        // TODO
        self.dump(&mut stdout());
        //let ordering = self.schedule(ScheduleOptimizationProfile::MaximizeAliasing);
        let ordering = self.graph.node_indices().collect::<Vec<_>>();
        let mut dot = File::create("graph.dot").unwrap();
        self.dump_graphviz(&mut dot, Some(&ordering), false);
    }
}

//--------------------------------------------------------------------------------------------------
// INTERNAL API
impl Frame {
    /// Creates a new frame.
    fn new(device: &Arc<Device>) -> Frame {
        let mut f = Frame {
            device: device.clone(),
            graph: FrameGraph::new(),
            images: IdVec::new(),
            buffers: IdVec::new(),
        };
        f
    }

    fn image_resource_mut(&mut self, id: ImageId) -> &mut ImageResource {
        &self.images[id]
    }

    fn buffer_resource_mut(&mut self, id: BufferId) -> &mut BufferResource {
        &self.buffers[id]
    }

    /// Creates a new task that will execute on the specified queue.
    /// Returns the ID to the newly created task.
    fn create_pass(&mut self, pass: impl Pass) -> PassId
        where T: Pass + 'static
    {
        self.graph.add_node(Box::new(task))
    }

    fn set_task<T>(&mut self, id: PassId, task: T)
        where T: Pass + 'static
    {
        *self.graph.node_weight_mut(id).unwrap() = Box::new(task);
    }


}
