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
use context::VkDevice1;

use ash::version::DeviceV1_0;
use petgraph::algo::{has_path_connecting, toposort, DfsSpace};
use petgraph::graph::{EdgeIndex, EdgeReference};
use petgraph::visit::{GraphBase, VisitMap, Visitable};
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
                        }).or_insert(nord);
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
pub(crate) struct TaskGroup {
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

/*fn subgraph_externals(g: &FrameGraph, sub: &[TaskId], direction: Direction) -> impl Iterator<Item=TaskId>
{
    sub.iter().filter(|n| {
        g.edges_directed(n, direction).all( |e| {
            let nn = match direction {
                Direction::Incoming => { e.source() },
                Direction::Outgoing => { e.target() }
            };
            !sub.contains(&nn)
        })
    })
}*/

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

/// Finds cross-queue synchronization edges and filter the redundant ones, and creates semaphores for all of them.
fn find_cross_queue_sync_edges(g: &FrameGraph, ordering: &[TaskId]) -> Vec<EdgeIndex<u32>> {
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

    debug!("sync edges after simplification:");
    for &s in syncs.iter() {
        let (a, b) = g.edge_endpoints(s).unwrap();
        debug!("{} -> {}", a.index(), b.index());
    }

    syncs
}

/// Creates task groups and semaphores between task groups.
fn create_task_groups(
    g: &FrameGraph,
    ordering: &[TaskId],
    syncs: &[EdgeIndex<u32>],
    vkd: &VkDevice1,
) -> (Vec<TaskGroup>, Vec<vk::Semaphore>) {
    // create semaphores (one per cross-queue edge)
    let mut semaphores = syncs
        .iter()
        .map(|s| {
            let create_info = vk::SemaphoreCreateInfo {
                s_type: vk::StructureType::SemaphoreCreateInfo,
                p_next: ptr::null(),
                flags: vk::SemaphoreCreateFlags::empty(),
            };
            unsafe { vkd.create_semaphore(&create_info, None).unwrap() }
        }).collect::<Vec<_>>();

    let mut pending_task_groups = Vec::new();
    for i in 0..3 {
        pending_task_groups.push(TaskGroup {
            tasks: Vec::new(),
            wait_semaphores: Vec::new(),
            signal_semaphores: Vec::new(),
            queue: i as u32,
        });
    }
    let mut task_groups = Vec::new();
    let mut visited = g.visit_map();

    //
    let flush_task_group = |pending: &mut TaskGroup, all_task_groups: &mut Vec<TaskGroup>| {
        if !pending.tasks.is_empty() {
            let queue = pending.queue;
            let t = mem::replace(
                pending,
                TaskGroup {
                    tasks: Vec::new(),
                    wait_semaphores: Vec::new(),
                    signal_semaphores: Vec::new(),
                    queue,
                },
            );
            all_task_groups.push(t);
        }
    };

    for &t in ordering.iter() {
        let queue_index = g.node_weight(t).unwrap().queue as usize;
        let mut wait_semaphores = Vec::new();
        let mut signal_semaphores = Vec::new();
        for (i, &s) in syncs.iter().enumerate() {
            let (a, b) = g.edge_endpoints(s).unwrap();
            if b == t {
                wait_semaphores.push(semaphores[i]);
            }
            if a == t {
                signal_semaphores.push(semaphores[i]);
            }
        }

        if !wait_semaphores.is_empty() {
            // terminate command buffer immediately, and start another one.
            flush_task_group(&mut pending_task_groups[queue_index], &mut task_groups);
            pending_task_groups[queue_index].wait_semaphores = wait_semaphores;
        }

        // add this task, and any synchronization requirements
        pending_task_groups[queue_index].tasks.push(t);

        if !signal_semaphores.is_empty() {
            // terminate command buffer
            pending_task_groups[queue_index].signal_semaphores = signal_semaphores;
            flush_task_group(&mut pending_task_groups[queue_index], &mut task_groups);
        }
    }

    // terminate all remaining task groups.
    for tg in pending_task_groups.iter_mut() {
        flush_task_group(tg, &mut task_groups);
    }

    // debug
    for tg in task_groups.iter() {
        print!("Task group: ");
        for &t in tg.tasks.iter() {
            let name = &g.node_weight(t).unwrap().name;
            print!("{} ", name);
        }
        print!(" | W:");
        for &s in tg.wait_semaphores.iter() {
            print!("{:?} ", s);
        }
        print!(" | S:");
        for &s in tg.signal_semaphores.iter() {
            print!("{:?} ", s);
        }
        println!();
    }

    (task_groups, semaphores)
}

/// Groups compatible tasks into renderpasses.
fn create_renderpasses(g: &FrameGraph, ordering: &[TaskId], task_groups: &[TaskGroup]) {
    unimplemented!()
}

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

impl<'ctx> Frame<'ctx> {
       pub fn schedule(&mut self, opt: ScheduleOptimizationProfile) -> Vec<TaskId> {
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
        debug!("begin scheduling");

        let (t_ordering, ordering) = if opt == ScheduleOptimizationProfile::MaximizeAliasing {
            measure_time(|| minimal_linear_ordering(&self.graph))
        } else {
            (0, self.graph.node_indices().collect::<Vec<_>>())
        };

        let (t_cross_queue_sync, sync_edges) =
            measure_time(|| find_cross_queue_sync_edges(&self.graph, &ordering));

        let (t_task_groups, (task_groups, semaphores)) = measure_time(|| {
            create_task_groups(&self.graph, &ordering, &sync_edges, &self.context.vkd)
        });

        debug!("scheduling report:");
        debug!("ordering ..................... {}µs", t_ordering);
        debug!("cross-queue sync ............. {}µs", t_cross_queue_sync);
        debug!("task group partition ......... {}µs", t_task_groups);

        debug!("end scheduling");

        ordering
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
// DONE Do away with dummy nodes for resource creation:
// - clutters the graph with useless nodes, confuses scheduling.
// - initialize to the correct state on first use.
//
// Decouple dependency edges and usage of the resource within the task.
// - A resource can have multiple usages within the same task.
//      - e.g. color attachment and input attachment
// - Dependency = only pipeline barrier info
