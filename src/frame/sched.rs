//! Frame scheduling and resource allocation.
//!
//! Handles many things:
//! * calculates the lifetime of resources
//! * infers the required usage flags of resources and pipeline barriers
//! * creates and schedules renderpasses
//!

use std::cell::RefCell;
use std::collections::HashMap;

use super::*;

use petgraph::algo::toposort;
use petgraph::visit::{VisitMap, Visitable};
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
        count + g.neighbors_directed(n, Direction::Outgoing).filter(|nn| !sub.contains(nn)).count() as u32
    )
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
        }).max()
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
                }).chain(
                    // also consider incoming externals that are not already in the set
                    g.externals(Direction::Incoming)
                        .filter(|nn| !sub.contains(nn)),
                ).for_each(|n| {
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
                    if ncut <= max_rank {
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
                            }).or_insert(nord);
                    }
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
pub(crate) struct TaskGroup
{
    /// DOC subgraph.
    tasks: Vec<TaskId>,
    /// DOC Semaphores to wait.
    wait_semaphores: Vec<vk::Semaphore>,
    /// DOC Semaphores to signal.
    signal_semaphores: Vec<vk::Semaphore>,
}

type TaskGroupId = u32;

/*fn create_task_groups_rec(
    n: TaskId,
    g: &FrameGraph,
    current_task_group: &mut TaskGroup,
    task_groups: &mut Vec<Option<TaskGroup>>) -> TaskGroup
{
    // create task group
    let mut group = TaskGroup {
        tasks: Vec::new(),
        wait_semaphores: Vec::new(),
        signal_semaphores: Vec::new()
    };

    g.edges_directed(root)
}*/

/*
/// Check if the subgraph extended by the given node is still a valid task group subgraph.
fn extend_task_group_subgraph(g: &FrameGraph, task_group: &[TaskId], new_tasks: &[TaskId])
{

}*/

impl<'ctx> Frame<'ctx> {
    fn collect_resource_usages(&mut self) {
        for d in self.graph.edge_references() {
            let d = d.weight();
            match d.details {
                DependencyDetails::Image {
                    id,
                    new_layout,
                    usage,
                    ref attachment,
                } => {
                    match &mut self.images[id.0 as usize] {
                        FrameResource::Transient {
                            ref mut description,
                            ..
                        } => {
                            // update usage flags
                            description.create_info.usage |= usage;
                        }
                        FrameResource::Imported { resource } => {
                            // TODO check usage flags
                        }
                    }
                }
                DependencyDetails::Buffer { id, usage } => {
                    match &mut self.buffers[id.0 as usize] {
                        FrameResource::Transient {
                            ref mut description,
                            ..
                        } => {
                            // update usage flags
                            description.create_info.usage |= usage;
                        }
                        FrameResource::Imported { resource } => {
                            // TODO check usage flags
                        }
                    }
                }
                DependencyDetails::Sequence => {}
            }
        }

        // FIXME add the VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT if the image is never accessed as
        // an image, sampled, or accessed by the host.
        // also, should add the VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT to the allocation.
    }

    fn create_semaphores(&mut self) 
    {
        //let semaphores = Vec::new();
        // look for every cross-queue dependency
        self.graph.edge_references()
            .filter(|e| {
                let d = e.weight();
                let t_src = self.graph.node_weight(e.source()).unwrap();
                let t_dst = self.graph.node_weight(e.target()).unwrap();
                t_src.queue.is_some() != t_dst.queue.is_some()      // FIXME ash upstream
            })
            .for_each(|e| {
                debug!("Cross-queue dependency: ID:{} -> ID:{}", e.source().index(), e.target().index());
            });
    }

    fn create_task_groups(&mut self) -> Vec<TaskGroup>
    {
        unimplemented!()
        // start with a node, assign it to a group
        // if one edge goes out of the queue, end group.
        // if one edge joins another group in the same queue, merge current group into the queue.
    }

    pub fn schedule(&mut self) -> Vec<TaskId> {
        // avoid toposort here, because the algo in petgraph
        // produces an ordering that is not optimal for aliasing.
        // Instead, compute the "directed minimum linear arrangement" (directed minLA)
        // of the execution graph.
        // This gives (I think) a task order that leads to better memory aliasing.
        // Note: the directed minLA problem is NP-hard, but seems to be manageable
        // in most cases?
        debug!("begin scheduling");
        let (t_ordering, result) = measure_time(|| minimal_linear_ordering(&self.graph));

        let (t_resource_usages, ()) = measure_time(|| {
            self.collect_resource_usages();
        });

        let (t_cross_queue_sync, ()) = measure_time(|| {
            self.create_semaphores();
        });

        debug!("scheduling report:");
        debug!("linear arrangement ........... {}µs", t_ordering);
        debug!("resource usage collection .... {}µs", t_resource_usages);
        debug!("cross-queue sync ............. {}µs", t_cross_queue_sync);

        debug!("end scheduling");

        result
    }
}

// Graph:
// Node = pass type (graphics or compute), callback function
// callback function parameters:
//  - command buffer, within a subpass instance initialized with the correct attachments
//  - container object that holds all requested resources in their correct state (all images, sampled images, buffer, storage buffer, uniform buffers, etc.)
// The callback just has to submit the draw command.
// Edge = dependency
// - toposort
// - check for concurrent read/write hazards (ok by API design)
// - infer usage of resources from graphs (mutate graph)
// - schedule renderpasses
// - reorder to minimize layout transitions
// - allocate resources with aliasing
// - group in render passes (draw commands with the same attachments; notably: chains without `sample` dependencies)
//      - new dependency type: attachment input
//      - at least one subpass for each different attachment?
//      - minimize the number of attachments
//      - a sample dependency always breaks the render pass
// - heuristic for renderpasses:
//      - schedule a pass (starting from the first one)
//      - follow all output attachments
//      - if no successor has a sample-dependency, and NO OTHER ATTACHMENTS, then merge the successors into the renderpass
//      - schedule_renderpasses()
//      - create dependencies between subpasses (e.g. output-to-input attachment)
//      - user-provided hints?
// - schedule renderpasses (dependencies between renderpasses: e.g. layout transitions)
// - insert memory barriers
// - insert layout transitions
//
// Various graphs:
// - initial graph
// - graph with render passes
// - graph with render passes and explicit layout transitions

//
// All work on the GPU is done inside nodes in a frame.
// - DEVICE: Submission queue: when allocating a transient resource:
//      - find a block to allocate, try to sync on semaphore,
//		- if not yet signalled, then allocate a new block
//      - if failed (transient memory limit exceeded), then sync on a suitable block
// - DEVICE: Submission queue: when importing a resource: semaphore sync (may be in use by another queue)
// - UPLOADS: fence sync on frame N-max_frames_in_flight
//
// When using a resource, what to sync on?
// - Associate semaphore to resource
//
// Sometimes, a group of resources (Buffers, Images, Memory blocks) can share the same semaphore:
// - SyncResourceGroup
//
// A resource can belong to a SyncGroup? Rc<SyncGroup>? SyncGroupId?
// The SyncGroup is assigned when? on submission?
// all resources used during the construction of a command buffer should be recorded
//
// ```
// context.sync_group(frame, command_buffer, |sync_resources| {
//		// if resource
//		sync_resources.use(...)
// })
// ```
//
// Next step: insert queue barriers
// - for each node
//      - for each dependency
//          - if cross-queue dependency
//              - if already syncing on a resource of the same queue that is finished later: do nothing
//              - otherwise, if finished earlier: remove old sync from task and put it on the later task (allocate semaphore if needed)
// - tasks have list of semaphores to signal, and semaphores to wait
//
// Next step:
// - traverse each queue subgraph and regroup tasks into 'jobs' that do not have any dependency on other queues
// - for each job, collect wait/signal semaphores
// - don't forget to add semaphores for external dependencies
// - this is handled by the import tasks
// - import tasks: what do they do?
//      - execute on the specified queue
//      - synchronizes with the previous frame (semaphores in the resource): adds semaphore to the job
//      - be careful not to synchronize with the semaphore from 2 or 3 frames before!
// - should also have export/exit nodes?
//      - exit nodes for external resources: signal resource ready
//      - automatic insertion of exit tasks
// - for each job: ring buffer of MAX_FRAMES_IN_FLIGHT semaphores?
//      - no need, I think, can reuse the same semaphores
//      - same for external resources, AS LONG AS the wait is on the same queue
//          - if the wait changes queues, then all hope is lost...
//              - e.g. if the queue is empty, then the GPU might execute it immediately, but frame N-2 might not have finished
//                  and may signal the semaphore, which will launch the task on the new queue with old data
//          - solution: have multiple semaphores for external resources, always wait on semaphore for frame N-1 exit
//
// Synchronization of external resources:
// - issue: can change queues (don't assume that they are all used on the same queue)
// - can read on a queue, read on the other
// - exit tasks: put a semaphore in the command stream, to be waited on by the entry (import) task
//
// Limitation:
// - cannot sequence (multiple) reads followed by a write!
// - maybe it's possible: return another versioned handle from a read
// - or modify graph: R(0) -> T1 -> W(0) -> T4 will add an implicit dependency on T4 to T2,T3
//                    R(0) -> T2
//                    R(0) -> T3
//   -> this assumes breadth-first execution...
//    t1.read(r0);     // pending access = [t1] (into r0 ref)
//    t2.read(r0);     // pending access = [t1,t2]
//    t3.read(r0);     // pending access = [t1,t2,t3]
//    // now r0 has three readers: reclaim exclusive access
//    let r0 = frame.sync(r0);      // r0 is a fresh reference without any R/W count, contains an implicit dependency on t1, t2, t3
//    -> insert a virtual task that depends on the three nodes (i.e. not a resource dependency)
//    // o1 means write after o1, but what about o2 and o3? => must detect R/W hazard
//    t4.write(o1);             // will sync on t1, but it's not enough
//
//    -> OPTION: could force sequencing of reads, in addition to writes
//    -> to write a resource, must sync on all pending reads
//    -> SOLUTION: add special "sequence" dependencies
//
// Next step: build command buffers
// - for each job, create command buffer, traverse graph
//
// Put everything in the graph, including present operations
// - some nodes should only execute on a given queue (e.g. the present queue)
//
// Transfer queue:
// - upload data immediately to upload buffer
// - on schedule: to transfer queue: copy to resource
//
