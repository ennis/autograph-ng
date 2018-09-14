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

/*fn extend_subgraph(
    g: &FrameGraph,
    prev: &[TaskId],
    cost: u32,
    cut: u32,
) -> Vec<(Vec<TaskId>, PartialOrdering)> {
    let greedy = true;

    //----------------------------------------------------------------------------------------------
    // compute the set of all neighbors of the prev subgraph
    let mut visited = RefCell::new(g.visit_map());
    prev.iter()
        .flat_map(|&n| {
            // all neighbors that were not already visited and that go outwards prev
            // and have all their incoming edges inside the set
            g.neighbors_directed(n, Direction::Outgoing)
                .filter(|&nn| visited.borrow_mut().visit(nn))
                .filter(|nn| !prev.contains(nn))
                .filter(|&nn| {
                    g.neighbors_directed(nn, Direction::Incoming)
                        .all(|nnn| prev.contains(&nnn))
                })
        }).chain(
            // also consider incoming externals that are not already in the set
            g.externals(Direction::Incoming)
                .filter(|nn| !prev.contains(nn)),
        ).map(|n| {
            //
            let i = g.edges_directed(n, Direction::Incoming).count() as u32;
            let o = g.edges_directed(n, Direction::Outgoing).count() as u32;
            // new cut = cut - i + o
            let ncut = cut - i + o;
            // new cost = cost + ncut
            let mut sub = prev.to_vec();
            sub.push(n);
            (
                sub,
                PartialOrdering {
                    cost: cost + ncut,
                    cut: ncut,
                    right: n,
                },
            )
        }).collect()

    //----------------------------------------------------------------------------------------------
    // greedy version of the above (keep only new subgraphs that minimize the new cut)
    /*let mut visited = RefCell::new(g.visit_map());
    let (ncut, next) = prev
        .iter()
        .flat_map(|&n| {
            // all neighbors that were not already visited and that go outwards prev
            // and have all their incoming edges inside the set
            g.neighbors_directed(n, Direction::Outgoing)
                .filter(|&nn| visited.borrow_mut().visit(nn))
                .filter(|nn| !prev.contains(nn))
                .filter(|&nn| {
                    g.neighbors_directed(nn, Direction::Incoming)
                        .all(|nnn| prev.contains(&nnn))
                })
        }).chain(
            // also consider incoming externals that are not already in the set
            g.externals(Direction::Incoming)
                .filter(|nn| !prev.contains(nn)),
        ).map(|n| {
            //
            let i = g.edges_directed(n, Direction::Incoming).count() as u32;
            let o = g.edges_directed(n, Direction::Outgoing).count() as u32;
            (cut - i + o, n)
        }).min_by(|a, b| a.0.cmp(&b.0))
        .unwrap();

    let mut sub = prev.to_vec();
    sub.push(next);
    vec![(ncut, sub)]*/

    //----------------------------------------------------------------------------------------------
    // greedy version of the above (keep only new subgraphs that minimize the new cut)
    /*let r = g
        .node_indices()
        .filter(|n| !prev.contains(n))
        .filter(|&n| {
            g.neighbors_directed(n, Direction::Incoming)
                .all(|nn| prev.contains(&nn))
        }).map(|n| {
            let i = g.edges_directed(n, Direction::Incoming).count() as u32;
            let o = g.edges_directed(n, Direction::Outgoing).count() as u32;
            (cut - i + o, n)
        }).min_by(|a, b| a.0.cmp(&b.0));

    let mut sub = prev.to_vec();
    if let Some((ncut, next)) = r {
        sub.push(next);
        vec![(ncut, sub)]
    } else {
        vec![]
    }*/

    //----------------------------------------------------------------------------------------------
    // this version is a bit slower
    /*g.node_indices()
        .filter(|n| !prev.contains(n))
        .filter(|&n| {
            g.neighbors_directed(n, Direction::Incoming)
                .all(|nn| prev.contains(&nn))
        }).map(|n| {
        // we know the cut of prev: quick way
        // to calculate the cut prev + n
        let i = g.neighbors_directed(n, Direction::Incoming).count() as u32;
        let o = g.neighbors_directed(n, Direction::Outgoing).count() as u32;
        let mut sub = prev.to_vec();
        sub.push(n);
        (cut - i + o, sub)
    }).collect()*/}
*/

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
        let subs = t.keys().filter(|sub| sub.len() == i).cloned().collect::<Vec<Vec<_>>>();
        debug!(
            "scheduling: calculating partial orderings of size {} ({} starting subsets)...",
            i+1,
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
        println!("size {} cost {}", sub.len(), ord.cost);
        minimal_ordering.push(ord.right);
        sub.remove_item(&ord.right);
    }

    minimal_ordering.reverse();

    println!("scheduling: minimal ordering found:");
    for n in minimal_ordering.iter() {
        print!("{},", n.index());
    }
    println!();
    println!();

    minimal_ordering
}

impl<'ctx> Frame<'ctx> {

    fn collect_resource_usages(&mut self)
    {
        for d in self.graph.edge_references()
        {
            let d = d.weight();
            match d.details {
                DependencyDetails::Image {
                    id,
                    new_layout,
                    usage,
                    ref attachment,
                } => {
                    match &mut self.images[id.0 as usize] {
                         FrameResource::Transient { ref mut description, .. } => {
                             // update usage flags
                             description.create_info.usage |= usage;
                         },
                         FrameResource::Imported { resource } => {
                             // TODO check usage flags
                         }
                    }
                },
                DependencyDetails::Buffer {
                    id, usage
                } => {
                    match &mut self.buffers[id.0 as usize] {
                        FrameResource::Transient { ref mut description, .. } => {
                            // update usage flags
                            description.create_info.usage |= usage;
                        },
                        FrameResource::Imported { resource } => {
                            // TODO check usage flags
                        }
                    }
                }
            }
        }

        // FIXME add the VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT if the image is never accessed as
        // an image, sampled, or accessed by the host.
        // also, should add the VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT to the allocation.
        unimplemented!()
    }

    pub fn schedule(&mut self) -> Vec<TaskId> {
        // FIXME avoid toposort here, because the algo in petgraph
        // produces an ordering that is not optimal for aliasing.
        // Instead, assume that the creation order specified by the user is better.
        // The topological ordering is checked on-the-fly during task submission.
        //
        // Note: the optimal solution to this problem is given by the
        // "directed minimum linear arrangement" (directed minLA), and is also related
        // to "minimum storage-time sequencing".
        // However, this is an NP-hard problem.


        let (st, result) = measure_time(|| {
            minimal_linear_ordering(&self.graph)
        });
        debug!("graph scheduling took {}µs", st);
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
