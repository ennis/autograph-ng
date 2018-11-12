//! Frame scheduling and resource allocation.
//!
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::ptr;

use crate::device::VkDevice1;
use crate::frame::dependency::{
    BarrierDetail, BufferBarrier, Dependency, ImageBarrier, SubpassBarrier,
};
use crate::frame::graph::{DependencyId, FrameGraph, TaskId};
use crate::frame::resource::{BufferId, BufferResource, ImageId, ImageResource};
use crate::frame::tasks::Pass;
use crate::frame::Frame;

use ash::version::DeviceV1_0;
use ash::vk;
use petgraph::algo::{has_path_connecting, toposort, DfsSpace};
use petgraph::graph::{EdgeIndex, EdgeReference};
use petgraph::visit::{EdgeRef, GraphBase, IntoEdgeReferences, VisitMap, Visitable};
use petgraph::Direction;
use sid_vec::FromIndex;
use time;

pub fn measure_time<R, F: FnOnce() -> R>(f: F) -> (u64, R) {
    let start = time::PreciseTime::now();
    let r = f();
    let duration = start.to(time::PreciseTime::now());
    (duration.num_microseconds().unwrap() as u64, r)
}

#[derive(Copy, Clone, Debug)]
struct PartialOrdering {
    /// Total cost of the ordering
    cost: u32,
    /// Cut
    cut: u32,
    /// Rightmost item
    right: TaskId,
}

fn subgraph_cut(g: &FrameGraph, sub: &[TaskId]) -> u32 {
    sub.iter().fold(0, |count, &n|
        // all outgoing neighbors that do not end up in the set
        count + g.neighbors_directed(n, Direction::Outgoing).filter(|nn| !sub.contains(nn)).count() as u32)
}

fn minimal_linear_ordering(g: &FrameGraph) -> Vec<TaskId> {
    let n = g.node_count();

    let mut t = HashMap::new();
    // init with externals
    for task in g.externals(Direction::Incoming) {
        let cut = subgraph_cut(g, &[task]);
        t.insert(
            vec![task],
            PartialOrdering {
                cost: 0,
                cut,
                right: task,
            },
        );
    }

    // calculate the highest node rank (num incoming + outgoing edges).
    // used to filter suboptimal solutions early.
    let max_rank = g
        .node_indices()
        .map(|n| {
            g.edges_directed(n, Direction::Outgoing).count()
                + g.edges_directed(n, Direction::Incoming).count()
        })
        .max()
        .unwrap() as u32;
    debug!("scheduling: max_rank = {}", max_rank);

    // fill table containing the cost of the optimal arrangement of each subset of the graph.
    for i in 1..=n {
        // collect subgraphs to explore
        // must clone because we modify the hash map
        let subs = t
            .keys()
            .filter(|sub| sub.len() == i)
            .cloned()
            .collect::<Vec<Vec<_>>>();
        debug!(
            "scheduling: calculating partial orderings of size {} ({} starting subsets)...",
            i + 1,
            subs.len()
        );
        // update scores of partial orderings
        // in hash map: (score, rightmost vertex)
        for sub in subs.iter() {
            // cost and cut of the optimal ordering for subset sub
            let (cost, cut) = {
                let ord = t[sub];
                (ord.cost, ord.cut)
            };
            // enumerate new topological orderings of size i+1 from existing sub
            // somewhat slower version:
            // .filter(|n| !sub.contains(n))
            // .filter(|&n| {
            //      g.neighbors_directed(n, Direction::Incoming)
            //        .all(|nn| sub.contains(&nn))
            //  })
            let mut visited = RefCell::new(g.visit_map());
            sub.iter()
                .flat_map(|&n| {
                    // all neighbors that were not already visited and that go outwards prev
                    // and have all their incoming edges inside the set
                    g.neighbors_directed(n, Direction::Outgoing)
                        .filter(|&nn| visited.borrow_mut().visit(nn))
                        .filter(|nn| !sub.contains(nn))
                        .filter(|&nn| {
                            g.neighbors_directed(nn, Direction::Incoming)
                                .all(|nnn| sub.contains(&nnn))
                        })
                })
                .chain(
                    // also consider incoming externals that are not already in the set
                    g.externals(Direction::Incoming)
                        .filter(|nn| !sub.contains(nn)),
                )
                .for_each(|n| {
                    // build the new ordering for subset sub + {n},
                    // calculate the cost, and update the subset table if
                    // the ordering has a lower cost.
                    // FIXME external resources don't count! since they don't alias memory
                    // FIXME count unique resources, not edges
                    let i = g.edges_directed(n, Direction::Incoming).count() as u32;
                    let o = g.edges_directed(n, Direction::Outgoing).count() as u32;
                    let ncut = cut - i + o;
                    let ncost = cost + ncut;
                    // don't bother if it's already over max rank
                    //if ncut <= max_rank {
                    let mut nsub = sub.clone();
                    nsub.push(n);
                    nsub.sort();
                    let nord = PartialOrdering {
                        cost: cost + ncut,
                        cut: ncut,
                        right: n,
                    };
                    t.entry(nsub)
                        .and_modify(|e| {
                            if e.cost > nord.cost {
                                *e = nord;
                            }
                        })
                        .or_insert(nord);
                    //}
                });
        }
    }

    // recover minimal ordering
    let mut minimal_ordering = Vec::new();
    let mut sub = g.node_indices().collect::<Vec<_>>();
    sub.sort();

    while !sub.is_empty() {
        let ord = t.get(&sub).unwrap();
        //println!("size {} cost {}", sub.len(), ord.cost);
        minimal_ordering.push(ord.right);
        sub.remove_item(&ord.right);
    }

    minimal_ordering.reverse();
    minimal_ordering
}

/// A sequence of tasks belonging to the same queue that can be submitted in the same command buffer.
pub(crate) struct CommandBuffer {
    /// DOC subgraph.
    tasks: Vec<TaskId>,
    /// On which queue to submit.
    queue: u32,
    /// DOC Semaphores to wait.
    wait_semaphores: Vec<vk::Semaphore>,
    /// DOC Semaphores to signal.
    signal_semaphores: Vec<vk::Semaphore>,
}

type TaskGroupId = u32;

fn subgraph_externals<'a>(
    g: &'a FrameGraph,
    sub: &'a [TaskId],
    direction: Direction,
) -> impl Iterator<Item = TaskId> + 'a {
    sub.iter()
        .filter(move |&&n| {
            g.edges_directed(n, direction).all(|e| {
                let nn = match direction {
                    Direction::Incoming => e.source(),
                    Direction::Outgoing => e.target(),
                };
                !sub.contains(&nn)
            })
        })
        .map(|&n| n)
}

/// Incoming or outgoing nodes of a subgraph.
fn subgraph_neighbors<'a>(
    g: &'a FrameGraph,
    sub: &'a [TaskId],
    direction: Direction,
) -> impl Iterator<Item = TaskId> + 'a {
    let visited = RefCell::new(g.visit_map());
    sub.iter()
        .flat_map(|&n| {
            g.neighbors_directed(n, direction)
                .filter(|nn| !sub.contains(nn))
                .filter(|&nn| visited.borrow_mut().visit(nn))
        })
        .collect::<Vec<_>>()
        .into_iter() // must collect to avoid visited to escape
}

fn grow_subgraph<'a>(
    g: &'a FrameGraph,
    visited: &'a RefCell<impl VisitMap<TaskId>>,
) -> impl Iterator<Item = TaskId> + 'a {
    g.node_indices()
        .filter(move |n| !visited.borrow().is_visited(n))
        .filter(move |&n| {
            g.neighbors_directed(n, Direction::Incoming)
                .all(|nn| visited.borrow().is_visited(&nn))
        })
}

fn subgraph_inner_edges<'a>(
    g: &'a FrameGraph,
    sub: &'a [TaskId],
) -> impl Iterator<Item = EdgeReference<'a, Dependency>> + 'a {
    g.edge_references()
        .filter(move |e| sub.contains(&e.source()) && sub.contains(&e.target()))
}

/*/// Outgoing edges of a subgraph.
fn directed_edges_between<'a>(
    g: &'a FrameGraph,
    sub_a: &'a [TaskId],
    sub_b: &'a [TaskId],
) -> impl Iterator<Item = EdgeReference<'a, Dependency>> + 'a {
    sub_a.iter().flat_map(move |&n| {
        g.edges_directed(n, Direction::Outgoing)
            .filter(move |e| sub_b.contains(&e.target()))
    })
}

/// Incoming or outgoing nodes of a subgraph.
fn subgraph_neighbors<'a>(
    g: &'a FrameGraph,
    sub: &'a [TaskId],
    direction: Direction,
) -> impl Iterator<Item = TaskId> + 'a {
    let mut visited = RefCell::new(g.visit_map());
    //let mut a = RefCell::new(0);
    sub.iter().flat_map(move |&n| {
        let visited = &mut visited; // visited moved into inner closure
        g.neighbors_directed(n, direction)
            .filter(|&nn| visited.borrow_mut().visit(nn))
    })
}

fn check_single_entry_graph(g: &FrameGraph, sub_a: &[TaskId], sub_b: &[TaskId]) -> bool {
    let ta = g.node_weight(*sub_a.first().unwrap()).unwrap();
    let tb = g.node_weight(*sub_b.first().unwrap()).unwrap();
    if ta.queue != tb.queue {
        // FIXME waiting on ash upstream
        return false; // not the same queue, cannot merge
    }

    // the two subsets must be ordered.
    let edges_a_to_b = directed_edges_between(g, sub_a, sub_b).count();
    let edges_b_to_a = directed_edges_between(g, sub_b, sub_a).count();

    let (sub_src, sub_dst) = match (edges_a_to_b, edges_b_to_a) {
        (0, 0) => return false,                               // subsets not connected
        (0, n) => (sub_b, sub_a),                             // b > a
        (n, 0) => (sub_a, sub_b),                             // a < b
        _ => panic!("subsets are connected but not ordered"), // logic error: subsets must be either not connected or ordered
    };

    // check the single-entry property of the graph
    // the externals of sub_dst must be included in sub_src
    let src_incoming = subgraph_neighbors(g, sub_src, Direction::Incoming).collect::<Vec<_>>();
    let ok = subgraph_neighbors(g, sub_dst, Direction::Incoming).all(|n| src_incoming.contains(&n));
    ok
}*/

/*
/// Finds cross-queue synchronization edges and filter the redundant ones, and creates semaphores for all of them.
fn find_cross_queue_sync_edges(
    g: &FrameGraph,
    vkd: &VkDevice1,
) -> (Vec<EdgeIndex<u32>>, Vec<vk::Semaphore>) {
    let mut syncs = g
        .edge_references()
        .filter_map(|e| {
            let src = g.node_weight(e.source()).unwrap();
            let dst = g.node_weight(e.target()).unwrap();
            if src.queue != dst.queue {
                Some(e.id())
            } else {
                None
            }
        }).collect::<Vec<_>>();

    // remove redundant sync edges.
    debug!("sync edges before simplification:");
    for &s in syncs.iter() {
        let (a, b) = g.edge_endpoints(s).unwrap();
        debug!("{} -> {}", a.index(), b.index());
    }

    let mut i = 0;
    let mut len = syncs.len();
    while i < len {
        let remove = {
            let e_b = syncs[i];
            syncs
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .any(|(j, &e_a)| {
                    let (a_src, a_dst) = g.edge_endpoints(e_a).unwrap();
                    let (b_src, b_dst) = g.edge_endpoints(e_b).unwrap();
                    has_path_connecting(g, b_src, a_src, None)
                        && has_path_connecting(g, a_dst, b_dst, None)
                })
        };

        if remove {
            syncs.remove(i);
        } else {
            i += 1;
        }
        len = syncs.len();
    }

    /*debug!("sync edges after simplification:");
    for &s in syncs.iter() {
        let (a, b) = g.edge_endpoints(s).unwrap();
        debug!("{} -> {}", a.index(), b.index());
    }*/
// create semaphores (one per cross-queue edge)
let semaphores = syncs
.iter()
.map(|s| {
let create_info = vk::SemaphoreCreateInfo {
s_type: vk::StructureType::SemaphoreCreateInfo,
p_next: ptr::null(),
flags: vk::SemaphoreCreateFlags::empty(),
};
unsafe { vkd.create_semaphore(&create_info, None).unwrap() }
}).collect::<Vec<_>>();

(syncs, semaphores)
}
*/

/*
struct CommandBufferBuilder<'a, 'ctx: 'a> {
    g: &'a FrameGraphInner,
    syncs: &'a [DependencyId],
    semaphores: &'a [vk::Semaphore],
    resources: &'a Resources<'ctx>,
    pending_cmd_buffers: Vec<CommandBuffer>,
    cmd_buffers: Vec<CommandBuffer>,
    layouts: Vec<vk::ImageLayout>,
}

impl<'a, 'ctx: 'a> CommandBufferBuilder<'a, 'ctx> {
    fn new(
        g: &'a FrameGraphInner,
        syncs: &'a [DependencyId],
        semaphores: &'a [vk::Semaphore],
        resources: &'a Resources<'ctx>,
    ) -> CommandBufferBuilder<'a, 'ctx> {
        let mut pending_cmd_buffers = Vec::new();
        for i in 0..3 {
            pending_cmd_buffers.push(CommandBuffer {
                tasks: Vec::new(),
                wait_semaphores: Vec::new(),
                signal_semaphores: Vec::new(),
                queue: i as u32,
            });
        }
        let layouts = resources
            .images
            .iter()
            .map(ImageResource::initial_layout)
            .collect::<Vec<_>>();
        let cmd_buffers = Vec::new();
        CommandBufferBuilder {
            g,
            semaphores,
            syncs,
            resources,
            pending_cmd_buffers,
            cmd_buffers,
            layouts,
        }
    }

    fn collect_cross_queue_syncs(
        &self,
        t: TaskId,
        wait_semaphores: &mut Vec<vk::Semaphore>,
        signal_semaphores: &mut Vec<vk::Semaphore>,
    ) {
        for (i, &s) in self.syncs.iter().enumerate() {
            let (a, b) = self.g.edge_endpoints(s).unwrap();
            if b == t {
                wait_semaphores.push(self.semaphores[i]);
            }
            if a == t {
                signal_semaphores.push(self.semaphores[i]);
            }
        }
    }

    /// Terminates command buffer of the specified queue and begin a new one.
    fn queue_barrier(&mut self, queue: u32) {
        let mut pending = &mut self.pending_cmd_buffers[queue as usize];
        if !pending.tasks.is_empty() {
            let mut t = mem::replace(
                pending,
                CommandBuffer {
                    tasks: Vec::new(),
                    wait_semaphores: Vec::new(),
                    signal_semaphores: Vec::new(),
                    queue,
                },
            );
            self.cmd_buffers.push(t);
        }
    }

    /// Schedules the specified task on the given queue.
    fn enqueue_task(&mut self, queue: u32, task: TaskId) {
        let mut wait_semaphores = Vec::new();
        let mut signal_semaphores = Vec::new();
        self.collect_cross_queue_syncs(task, &mut wait_semaphores, &mut signal_semaphores);

        // have to wait on something: terminate command buffer immediately, and start another one
        if !wait_semaphores.is_empty() {
            self.queue_barrier(queue);
            self.pending_cmd_buffers[queue as usize].wait_semaphores = wait_semaphores;
        }

        self.pending_cmd_buffers[queue as usize].tasks.push(task);

        if !signal_semaphores.is_empty() {
            self.pending_cmd_buffers[queue as usize].signal_semaphores = signal_semaphores;
            self.queue_barrier(queue);
        }
    }

    /// Schedules the specified tasks in one go on the given queue (without inserting barriers).
    fn enqueue_tasks(&mut self, queue: u32, tasks: &[TaskId]) {
        let mut wait_semaphores = Vec::new();
        let mut signal_semaphores = Vec::new();

        for &task in tasks {
            self.collect_cross_queue_syncs(task, &mut wait_semaphores, &mut signal_semaphores);
        }

        if !wait_semaphores.is_empty() {
            self.queue_barrier(queue);
            self.pending_cmd_buffers[queue as usize].wait_semaphores = wait_semaphores;
        }

        self.pending_cmd_buffers[queue as usize]
            .tasks
            .extend(tasks.iter());

        if !signal_semaphores.is_empty() {
            self.pending_cmd_buffers[queue as usize].signal_semaphores = signal_semaphores;
            self.queue_barrier(queue);
        }
    }

    fn finalize(mut self) -> Vec<CommandBuffer> {
        // terminate all remaining task groups.
        for queue in 0..self.pending_cmd_buffers.len() {
            self.queue_barrier(queue as u32);
        }
        self.cmd_buffers
    }
}

const MAX_QUEUES: usize = 16;

fn schedule<'ctx>(
    g: &FrameGraph,
    renderpasses: &RenderPasses,
    syncs: &[DependencyId],
    semaphores: &[vk::Semaphore],
    resources: &Resources<'ctx>,
) -> Schedule {
    let mut to_schedule = g.externals(Direction::Incoming).collect::<VecDeque<_>>();
    let mut scheduled = RefCell::new(g.visit_map());
    let mut remaining = g.node_count();
    let mut cmd_builder = CommandBufferBuilder::new(g, syncs, semaphores, resources);
    let is_scheduled = |t| scheduled.borrow().is_visited(&t);

    while !to_schedule.is_empty() {
        let taskid = to_schedule.pop_front().unwrap();
        let task = g.node_weight(taskid).unwrap();
        let queue = task.queue;

        debug!("schedule of task {}(ID:{})", task.name, taskid.index());

        // is the task already scheduled? (can happen because of ahead-of-time schedule of renderpasses)
        if is_scheduled(taskid) {
            continue;
        }

        /*match task.details {
            TaskDetails::Graphics(ref graphics_task) => {
                let renderpass = &renderpasses[graphics_task.renderpass];
                // are all deps scheduled?
                if !subgraph_neighbors(g, &renderpass.tasks, Direction::Incoming)
                    .all(|t| is_scheduled(t))
                {
                    debug!("push back");
                    to_schedule.push_back(taskid);
                    continue;
                }

                cmd_builder.enqueue_tasks(queue, &renderpass.tasks);
                renderpass.tasks.iter().for_each(|&t| {
                    scheduled.borrow_mut().visit(t);
                });
                //remaining -= renderpass.tasks;
            }
            _ => {
                cmd_builder.enqueue_task(queue, taskid);
                scheduled.borrow_mut().visit(taskid);
                //remaining -= 1;
            }
        }*/
// grow the graph:
// push front = depth-first
// push back = breadth-first
grow_subgraph(g, &scheduled).for_each(|n| to_schedule.push_front(n));
}

let cmdbufs = cmd_builder.finalize();

// dump final ordering
for cmdbuf in cmdbufs.iter() {
print!("Command buffer: ");
for &t in cmdbuf.tasks.iter() {
let name = &g.node_weight(t).unwrap().name;
print!("{} ", name);
}
print!(" | W:");
for &s in cmdbuf.wait_semaphores.iter() {
print!("{:?} ", s);
}
print!(" | S:");
for &s in cmdbuf.signal_semaphores.iter() {
print!("{:?} ", s);
}
println!();
}
}
*/

/// Optimization profiles for scheduling.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum ScheduleOptimizationProfile {
    /// No reordering is performed, keep the submission order.
    NoOptimization,
    /// Nodes are reordered to maximize resource memory aliasing.
    MaximizeAliasing,
}

impl Default for ScheduleOptimizationProfile {
    fn default() -> Self {
        ScheduleOptimizationProfile::NoOptimization
    }
}

impl<'id> Frame<'id> {
    /*/// Inserts dummy tasks for all external resources that handle the synchronization.
    fn insert_exit_tasks(&mut self) {
        // find last uses of each external resource
        let tasks_to_create = self
            .images
            .iter()
            .enumerate()
            .filter(|(_, img)| !img.is_transient())
            .map(|(i, img)| {
                let i = ImageId::from_index(i);
                (i, self.graph.collect_last_uses_of_image(i))
            }).collect::<Vec<_>>();
    
        // add tasks
        for t in tasks_to_create.iter() {
            // on which queue?
            self.make_sequence_task("exit", &t.1);
        }
    }*/

    pub fn schedule(&mut self, opt: ScheduleOptimizationProfile) -> Vec<TaskId> {
        debug!("begin scheduling");

        //self.insert_exit_tasks();

        // avoid toposort here, because the algo in petgraph
        // produces an ordering that is not optimal for aliasing.
        // Instead, compute the "directed minimum linear arrangement" (directed minLA)
        // of the execution graph.
        // This gives (I think) a task order that leads to better memory aliasing.
        // Note: the directed minLA problem is NP-hard, but seems to be manageable
        // in most cases?
        //  "Optimizing the dependency graph for maximum overlap also greatly
        //   reduces the opportunities for aliasing, so if we want to take memory
        //   into consideration, this algorithm could easily get far more involved..."
        //      - http://themaister.net/blog/2017/08/15/render-graphs-and-vulkan-a-deep-dive/
        let (t_ordering, ordering) = if opt == ScheduleOptimizationProfile::MaximizeAliasing {
            measure_time(|| minimal_linear_ordering(&self.graph))
        } else {
            (0, self.graph.node_indices().collect::<Vec<_>>())
        };

        /* let (t_cross_queue_sync, (sync_edges, semaphores)) =
        measure_time(|| find_cross_queue_sync_edges(&self.graph.0, &self.context.vkd));*/

        let (t_scheduling, ()) = measure_time(|| {
            /*schedule(
                &self.graph.0,
                &self.renderpasses,
                &sync_edges,
                &semaphores,
                &self.resources,
            );*/
        });

        /*let (t_renderpasses, ()) = measure_time(|| {
            build_renderpasses(&self.renderpasses, &self.graph.0, &ordering, &task_groups)
        });*/

        debug!("scheduling report:");
        debug!("ordering ..................... {}µs", t_ordering);
        //debug!("cross-queue sync ............. {}µs", t_cross_queue_sync);
        //debug!("scheduling ................... {}µs", t_scheduling);
        //debug!("task group partition ......... {}µs", t_task_groups);
        //debug!("build renderpasses ........... {}µs", t_renderpasses);

        debug!("end scheduling");

        ordering
    }
}
