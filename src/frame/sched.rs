//! Frame scheduling and resource allocation.
//!
//! Handles many things:
//! * calculates the lifetime of resources
//! * infers the required usage flags of resources and pipeline barriers
//! * creates and schedules renderpasses
//!

use super::*;

use petgraph::algo::toposort;

impl<'ctx> Frame<'ctx>
{
    pub fn schedule(&mut self)
    {
        // issue: this is not an optimal toposort: we want to minimize the lifetime of resources
        // so that they can be aliased.
        // Solution: Directed Minimum Linear Arrangement of a Directed Graph

        let sorted = toposort(&self.graph, None).expect("Dependency graph has cycles");
        // print some info about the graph.
        info!("Frame info:");

        for n in sorted.iter() {
            let task = self.graph.node_weight(*n).unwrap();
            info!("  {}(#{})", task.name, n.index());
        }
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

