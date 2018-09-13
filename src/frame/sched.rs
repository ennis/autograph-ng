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

pub fn measure_time<F: FnOnce()>(f: F) -> u64 {
    let start = time::PreciseTime::now();
    f();
    let duration = start.to(time::PreciseTime::now());
    duration.num_microseconds().unwrap() as u64
}

fn extend_subgraph(g: &FrameGraph, prev: &[TaskId], cut: u32) -> Vec<(u32, Vec<TaskId>)> {
    let greedy = true;

    //----------------------------------------------------------------------------------------------
    // compute the set of all neighbors of the prev subgraph
    /*let mut visited = RefCell::new(g.visit_map());
    prev.iter().flat_map(|&n| {
        // all neighbors that were not already visited and that go outwards prev
        // and have all their incoming edges inside the set
        g.neighbors_directed(n, Direction::Outgoing)
            .filter(|&nn| visited.borrow_mut().visit(nn))
            .filter(|nn| !prev.contains(nn))
            .filter(|&nn| g.neighbors_directed(nn, Direction::Incoming).all(|nnn| prev.contains(&nnn)))
    }).chain(
        // also consider incoming externals that are not already in the set
        g.externals(Direction::Incoming).filter(|nn| !prev.contains(nn))
    ).map(|n| {
        //
        let i = g.edges_directed(n, Direction::Incoming).count() as u32;
        let o = g.edges_directed(n, Direction::Outgoing).count() as u32;
        //(cut - i + o, sub)
        let mut sub = prev.to_vec();
        sub.push(n);
        (cut - i + o, sub)
    }).collect()*/

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
    g.node_indices()
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
    }).collect()
}

fn subgraph_cut(g: &FrameGraph, sub: &[TaskId]) -> u32 {
    sub.iter().fold(0, |count, &n|
        // all outgoing neighbors that do not end up in the set
        count + g.neighbors_directed(n, Direction::Outgoing).filter(|nn| !sub.contains(nn)).count() as u32
    )
}

fn update_score(t: &mut HashMap<Vec<TaskId>, (u32, TaskId)>, sub: &[TaskId], score: u32) {
    let mut sorted = sub.to_vec();
    let last = sub.last().unwrap().clone();
    sorted.sort();
    t.entry(sorted)
        .and_modify(|e| {
            if e.0 > score {
                *e = (score, last);
            }
        }).or_insert((score, last));
}

fn minimal_linear_ordering(g: &FrameGraph) {
    let n = g.node_count();

    let mut t = HashMap::new();
    // init with externals
    for ext in g.externals(Direction::Incoming) {
        let score = subgraph_cut(g, &[ext]);
        t.insert(vec![ext], (score, ext));
    }

    /*for (k,v) in t.iter() {
        for n in k.iter() {
            print!("{},", n.index());
        }
        println!("COST={}", v.0);
    }*/

    // calculate the highest node rank (num incoming + outgoing edges).
    let max_rank = g
        .node_indices()
        .map(|n| {
            g.edges_directed(n, Direction::Outgoing).count()
                + g.edges_directed(n, Direction::Incoming).count()
        }).max()
        .unwrap() as u32;
    debug!("scheduling: max_rank = {}", max_rank);

    for i in 1..=n {
        debug!(
            "scheduling: calculating partial orderings of size {} ({} orderings)...",
            i,
            t.len()
        );
        // collect subgraphs to explore
        // must clone because we modify the hash map
        let subgs = t.keys().cloned().collect::<Vec<Vec<_>>>();
        // update scores of partial orderings
        // in hash map: (score, rightmost vertex)
        for sub in subgs.iter().filter(|sub| sub.len() == i) {
            let esubs = extend_subgraph(g, sub, t[sub].0);
            for esub in esubs.iter() {
                // calc score
                //let score = subgraph_cut(g, &esub.1);
                let score = esub.0;
                print!("ORDERING: ");
                for n in esub.1.iter() {
                    print!("{},", n.index());
                }
                println!(" SCORE: {}", esub.0);
                // update score in hash map
                // don't add the ordering to the map if it's already over max rank
                if score <= max_rank {
                    // this is expensive
                    update_score(&mut t, &esub.1, score);
                }
            }
        }

        /*// remove all entries that are already over max rank
        let size_before = t.len();
        t.retain(|k, v| v.0 <= max_rank);
        let num_culled = size_before - t.len();
        debug!(
            "scheduling: culled {}/{} partial orderings",
            num_culled, size_before
        );*/
    }


    // recover minimal ordering
    let mut minimal_ordering = Vec::new();
    let mut sub = g.node_indices().collect::<Vec<_>>();
    sub.sort();

    while !sub.is_empty() {
        let (cost,task) = t.get(&sub).unwrap();
        println!("size {} cost {}", sub.len(), cost);
        minimal_ordering.push(task);
        sub.remove_item(&task);
    }

    minimal_ordering.reverse();

    println!("scheduling: minimal ordering found:");
    for n in minimal_ordering.iter() {
        print!("{},", n.index());
    }
    println!();
    println!();
}

impl<'ctx> Frame<'ctx> {
    fn infer_image_usage(&self, img: ImageId) -> vk::ImageUsageFlags {
        // Collect all dependencies on this image.
        self.graph.edge_references().filter_map(|d| {
            let d = d.weight();
            match d.details {
                DependencyDetails::Attachment {
                    id,
                    index,
                    ref description,
                }
                    if id == img =>
                {
                    if d.access_bits.intersects(
                        vk::ACCESS_COLOR_ATTACHMENT_READ_BIT
                            | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                    ) {
                        Some(vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT)
                    } else if d.access_bits.intersects(
                        vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT
                            | vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT,
                    ) {
                        Some(vk::IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT)
                    } else if d
                        .access_bits
                        .intersects(vk::ACCESS_INPUT_ATTACHMENT_READ_BIT)
                    {
                        Some(vk::IMAGE_USAGE_INPUT_ATTACHMENT_BIT)
                    } else {
                        None
                    }
                }
                DependencyDetails::Image { id, new_layout } if id == img => {
                    if d.access_bits
                        .intersects(vk::ACCESS_SHADER_READ_BIT | vk::ACCESS_SHADER_WRITE_BIT)
                    {
                        Some(vk::IMAGE_USAGE_STORAGE_BIT)
                    } else {
                        Some(vk::IMAGE_USAGE_SAMPLED_BIT)
                    }
                }
                _ => None,
            }
        });

        // add the VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT if the image is never accessed as
        // an image, sampled, or accessed by the host.
        // also, should add the VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT to the allocation.
        unimplemented!()
    }

    pub fn schedule(&mut self) {
        // FIXME avoid toposort here, because the algo in petgraph
        // produces an ordering that is not optimal for aliasing.
        // Instead, assume that the creation order specified by the user is better.
        // The topological ordering is checked on-the-fly during task submission.
        //
        // Note: the optimal solution to this problem is given by the
        // "directed minimum linear arrangement" (directed minLA), and is also related
        // to "minimum storage-time sequencing".
        // However, this is an NP-hard problem.

        //let sorted = toposort(&self.graph, None).expect("Dependency graph has cycles");

        // infer usage of resources:
        // * vk::AttachmentDescription::initial_layout (vk::ImageLayout)
        // * vk::ImageCreateInfo::tiling (vk::ImageTiling)
        //      (use optimal)
        // * vk::ImageCreateInfo::usage (vk::ImageUsageFlags)
        //      (look for all transitive dependencies, starting from the creation node)
        // * vk::ImageCreateInfo::initial_layout (vk::ImageLayout)
        let st = measure_time(|| {
            minimal_linear_ordering(&self.graph);
        });
        info!("scheduling took {}µs", st);

        // variant 1: 8_316_419 µs
        // variant 2: 7_367_404 µs (score calculated on the fly)

        /*info!("Frame info:");
        for n in sorted.iter() {
            let task = self.graph.node_weight(*n).unwrap();
            info!("  {}(#{})", task.name, n.index());
        }*/
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
