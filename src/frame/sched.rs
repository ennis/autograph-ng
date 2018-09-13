//! Frame scheduling and resource allocation.
//!
//! Handles many things:
//! * calculates the lifetime of resources
//! * infers the required usage flags of resources and pipeline barriers
//! * creates and schedules renderpasses
//!

use std::collections::HashMap;

use super::*;

use petgraph::algo::toposort;


fn extend_subgraph(g: &FrameGraph, prev: &[TaskId]) -> Vec<Vec<TaskId>>
{
    g.node_indices()
        .filter(|n| !prev.contains(n))
        .filter(|&n|
            g.neighbors_directed(n, Direction::Incoming).all(|nn| prev.contains(&nn))
        )
        .map(|n| {
            let mut subg = prev.to_vec();
            subg.push(n);
            subg
        })
        .collect()
}

fn subgraph_cut(g: &FrameGraph, sub: &[TaskId]) -> u32
{
    sub.iter().fold(0, |count, &n|
        // all outgoing neighbors that do not end up in the set
        count + g.neighbors_directed(n, Direction::Outgoing).filter(|nn| !sub.contains(nn)).count() as u32
    )
}

fn update_score(t: &mut HashMap<Vec<TaskId>, (u32,TaskId)>, sub: &[TaskId], score: u32)
{
    let mut sorted = sub.to_vec();
    let last = sub.last().unwrap().clone();
    sorted.sort();
    t.entry(sorted)
        .and_modify(|e| {
            if e.0 > score {
                *e = (score, last);
            }
        })
        .or_insert((score, last));
}

fn minimal_linear_ordering(g: &FrameGraph)
{
    let n = g.node_count();

    let mut t = HashMap::new();
    // init with externals
    for ext in g.externals(Direction::Incoming) {
        t.insert(vec![ext], (1, ext));
    }

    for i in 1..=n {
        println!("Calculating partial orderings of size {}...", i);
        // collect subgraphs to explore
        // must clone because we modify the hash map
        let subgs = t.keys().cloned().collect::<Vec<Vec<_>>>();
        // update scores of partial orderings
        // in hash map: (score, rightmost vertex)
        for sub in subgs.iter() {
            let esubs = extend_subgraph(g, sub);
            for esub in esubs.iter() {
                // calc score
                let score = subgraph_cut(g, esub);
                // update score in hash map
                update_score(&mut t, esub, score);
            }
        }
    }

    for (k,v) in t.iter() {
        for n in k.iter() {
            print!("{},", n.index());
        }
        println!("COST={}", v.0);
    }

    // recover minimal ordering
    let mut minimal_ordering = Vec::new();
    let mut sub = g.node_indices().collect::<Vec<_>>();
    sub.sort();

    while !sub.is_empty() {
        let task = t.get(&sub).unwrap().1;
        minimal_ordering.push(task);
        sub.remove_item(&task);
    }

    minimal_ordering.reverse();

    println!("MINIMAL ORDERING:");
    for n in minimal_ordering.iter() {
        print!("{},", n.index());
    }
    println!();
    println!();
}

impl<'ctx> Frame<'ctx>
{

    fn infer_image_usage(&self, img: ImageId) -> vk::ImageUsageFlags
    {
        // Collect all dependencies on this image.
        self.graph.edge_references().filter_map(|d| {
            let d = d.weight();
            match d.details {
                DependencyDetails::Attachment { id, index, ref description } if id == img => {
                    if d.access_bits.intersects(vk::ACCESS_COLOR_ATTACHMENT_READ_BIT | vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT) {
                        Some(vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT)
                    } else if d.access_bits.intersects(vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT | vk::ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT) {
                        Some(vk::IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT)
                    } else if d.access_bits.intersects(vk::ACCESS_INPUT_ATTACHMENT_READ_BIT) {
                        Some(vk::IMAGE_USAGE_INPUT_ATTACHMENT_BIT)
                    } else {
                        None
                    }
                },
                DependencyDetails::Image { id, new_layout } if id == img => {
                    {
                        if d.access_bits.intersects(vk::ACCESS_SHADER_READ_BIT | vk::ACCESS_SHADER_WRITE_BIT) {
                            Some(vk::IMAGE_USAGE_STORAGE_BIT)
                        } else {
                            Some(vk::IMAGE_USAGE_SAMPLED_BIT)
                        }
                    }
                },
                _ => None,
            }
        });

        // add the VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT if the image is never accessed as
        // an image, sampled, or accessed by the host.
        // also, should add the VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT to the allocation.
        unimplemented!()
    }

    pub fn schedule(&mut self)
    {
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
        minimal_linear_ordering(&self.graph);

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

