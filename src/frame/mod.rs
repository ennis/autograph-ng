//! Frame graphs
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::fs::File;
use std::io::{stdout, Write};
use std::marker::PhantomData;
use std::mem;
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
mod graph;
mod renderpass;
mod resource;
pub mod tasks;

use crate::device::Device;

use self::dependency::*;
use self::graph::*;
use self::renderpass::*;
use self::resource::buffer::*;
use self::resource::image::*;
use self::resource::*;
use self::tasks::*;

pub use self::tasks::present::*;
pub use self::tasks::Task;

//--------------------------------------------------------------------------------------------------
type LifetimeId<'id> = PhantomData<Cell<&'id mut ()>>;

//--------------------------------------------------------------------------------------------------
/// A frame: manages transient resources within a frame.
/// 'id is an invariant lifetime that should be used to tag resource references (ImageRefs and BufferRefs)
pub struct Frame<'id, 'imp> {
    device: Arc<Device>,
    graph: FrameGraph,
    images: IdVec<ImageId, Box<ImageResource + 'imp>>,
    buffers: IdVec<BufferId, Box<BufferResource + 'imp>>,
    renderpasses: IdVec<RenderPassId, RenderPass>,
    _marker: LifetimeId<'id>,
}

impl<'id, 'imp> Frame<'id, 'imp> {
    /// Creates a new frame.
    fn new(device: &Arc<Device>) -> Frame<'id, 'imp> {
        let mut f = Frame {
            device: device.clone(),
            graph: FrameGraph::new(),
            images: IdVec::new(),
            buffers: IdVec::new(),
            renderpasses: IdVec::new(),
            _marker: PhantomData,
        };
        f
    }

    /// Creates a present task.
    /// The input must be an imported swapchain image.
    pub fn present(&mut self, img: &ImageRef<'id>) {
        //let queue = self.context.present_queue;
        let mut builder = PresentTaskBuilder::new(self);
        builder.present(img);
        builder.finish();
    }

    /// Creates a task that has a dependency on all the specified tasks.
    fn make_sequence_task(&mut self, name: impl Into<String>, tasks: &[TaskId]) -> TaskId {
        // create the sync task
        let dst_task = self.create_task(DummyTask);
        for &src_task in tasks.iter() {
            self.add_sequence_dependency(src_task, dst_task);
        }
        dst_task
    }

    /// Imports a persistent image for use in the frame graph.
    pub fn import_image<I: ImageResource + Clone + 'imp>(&mut self, img: I) -> ImageRef<'id> {
        let task = self.create_task(DummyTask);
        let image = self.images.push(Box::new(img.clone()));

        ImageRef::new(
            image,
            task,
            vk::PIPELINE_STAGE_BOTTOM_OF_PIPE,
            vk::ACCESS_MEMORY_WRITE_BIT,
            img.layout(),
            0,
        )
    }

    /// Creates a new task that will execute on the specified queue.
    /// Returns the ID to the newly created task.
    fn create_task(&mut self, task: impl Task + 'static) -> TaskId {
        self.graph.add_node(Box::new(task))
    }

    fn set_task(&mut self, id: TaskId, task: impl Task + 'static) {
        *self.graph.node_weight_mut(id).unwrap() = Box::new(task);
    }

    /*pub(super) fn image_barrier(
        &mut self,
        img: &ImageRef,
        dst: TaskId,
        new_layout: ImageLayout,
        dst_stage_mask: PipelineStages,
        dst_access_mask: AccessFlagBits,
        latency: u32,
    ) {
        self.add_dependency(
            img.task(),
            dst,
            Dependency {
                src_stage_mask: img.src_stage_mask(),
                dst_stage_mask: PipelineStages {
                    vertex_shader: true,
                    ..PipelineStages::none()
                },
                barrier: BarrierDetail::Image(ImageBarrier {
                    id: img.id(),
                    old_layout: ImageLayout::Undefined,
                    new_layout: ImageLayout::ShaderReadOnlyOptimal,
                    src_access_mask: AccessFlagBits::none(),
                    dst_access_mask: AccessFlagBits {
                        shader_read: true,
                        ..AccessFlagBits::none()
                    },
                }),
                latency: img.latency(),
            },
        );
    }*/

    /// Adds or updates a dependency between two tasks in the graph.
    fn add_dependency(&mut self, src: TaskId, dst: TaskId, dependency: Dependency) -> DependencyId {
        // look for an already existing dependency
        if let Some(edge) = self.graph.find_edge(src, dst) {
            let dep = self.graph.edge_weight_mut(edge).unwrap();

            match (&mut dep.barrier, &dependency.barrier) {
                // buffer barrier
                (
                    &mut BarrierDetail::Buffer(ref mut barrier_a),
                    &BarrierDetail::Buffer(ref barrier_b),
                )
                    if barrier_a.id == barrier_b.id =>
                {
                    dep.src_stage_mask |= dependency.src_stage_mask;
                    dep.dst_stage_mask |= dependency.dst_stage_mask;
                    barrier_a.src_access_mask |= barrier_b.src_access_mask;
                    barrier_a.dst_access_mask |= barrier_b.dst_access_mask;
                    dep.latency = dep.latency.max(dependency.latency);
                    return edge;
                }
                // image barrier
                (
                    &mut BarrierDetail::Image(ref mut barrier_a),
                    &BarrierDetail::Image(ref barrier_b),
                )
                    if barrier_a.id == barrier_b.id =>
                {
                    dep.src_stage_mask |= dependency.src_stage_mask;
                    dep.dst_stage_mask |= dependency.dst_stage_mask;
                    barrier_a.src_access_mask |= barrier_b.src_access_mask;
                    barrier_a.dst_access_mask |= barrier_b.dst_access_mask;
                    // must be a compatible layout
                    dep.latency = dep.latency.max(dependency.latency);
                    return edge;
                }
                // FIXME subpass barrier on an attachment reference: merge with existing dependency?
                _ => {}
            }
        }

        // new dependency
        self.graph.add_edge(src, dst, dependency)
    }

    /// Adds a sequencing constraint between two nodes.
    /// A sequencing constraint does not involve any resource.
    fn add_sequence_dependency(&mut self, src: TaskId, dst: TaskId) -> DependencyId {
        self.add_dependency(
            src,
            dst,
            Dependency {
                src_stage_mask: vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                dst_stage_mask: vk::PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
                barrier: BarrierDetail::Sequence,
                latency: 0, // FIXME not sure...
            },
        )
    }

    /*/// Updates the "destination access mask" field of an image dependency.
    /// Panics if `dependency` is not an image dependency.
    fn add_image_barrier_access_flags(&mut self, dependency: DependencyId, flags: vk::AccessFlags) {
        self.0
            .edge_weight_mut(dependency)
            .unwrap()
            .as_image_barrier_mut()
            .unwrap()
            .dst_access_mask |= flags;
    }*/

    /// Collects all tasks using this resource but that do not produce another version of it.
    fn collect_last_uses_of_image(&self, img: ImageId) -> Vec<TaskId> {
        let uses = self
            .graph
            .node_indices()
            .filter(|n| {
                // is the resource used in an incoming dependency?
                let incoming = self
                    .graph
                    .edges_directed(*n, Direction::Incoming)
                    .any(|e| e.weight().get_image_id() == Some(img));
                // does not appear in any outgoing dependency
                let outgoing = self
                    .graph
                    .edges_directed(*n, Direction::Outgoing)
                    .any(|e| e.weight().get_image_id() == Some(img));

                incoming && !outgoing
            }).collect::<Vec<_>>();

        uses
    }

    /// Collects all tasks using this resource but that do not produce another version of it.
    fn collect_last_uses_of_buffer(&self, buf: BufferId) -> Vec<TaskId> {
        let uses = self
            .graph
            .node_indices()
            .filter(|n| {
                // is the resource used in an incoming dependency?
                let incoming = self
                    .graph
                    .edges_directed(*n, Direction::Incoming)
                    .any(|e| e.weight().get_buffer_id() == Some(buf));
                // does not appear in any outgoing dependency
                let outgoing = self
                    .graph
                    .edges_directed(*n, Direction::Outgoing)
                    .any(|e| e.weight().get_buffer_id() == Some(buf));

                incoming && !outgoing
            }).collect::<Vec<_>>();

        uses
    }

    pub fn submit(mut self) {
        // TODO
        //self.dump(&mut stdout());
        //let ordering = self.schedule(ScheduleOptimizationProfile::MaximizeAliasing);
        let mut dot = File::create("graph.dot").unwrap();
        //self.dump_graphviz(&mut dot, Some(&ordering), false);
    }
}

//--------------------------------------------------------------------------------------------------
// Context

/// Starts a frame on the specified device.
/// The frame handles the scheduling of all GPU operations, synchronization between
/// command queues, and synchronization with the CPU.
fn with_frame<'imp, F>(device: &Arc<Device>, closure: F)
where
    F: for<'id> FnOnce(&mut Frame<'id, 'imp>),
{
    let mut f = Frame::new(device);
    closure(&mut f);
}
