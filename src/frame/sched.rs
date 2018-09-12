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

impl<'ctx> Frame<'ctx>
{
    pub fn schedule(&mut self)
    {
        // We want a "good" toposort that minimizes the lifetimes of resources
        // to increase memory aliasing.
        // This is the "Directed Minimum Linear Arrangement" (Directed minLA) problem,
        // or the "minimum storage-time sequencing" problem.
        // However, finding an optimal solution is NP-hard,
        // so instead, do a stable toposort of the graph and hope that
        // the user did not add tasks in a weird order.
        // In most cases, the toposort shouldn't change anything.

        let sorted = toposort(&self.graph, None).expect("Dependency graph has cycles");

        /*let mut node_x = HashMap::new();

        // init positions
        for (i,s) in sorted.iter().enumerate() {
            node_x[s] = i as f32;
        }

        // damping
        let K = 0.1;

        // calculate forces
        let mut forces = vec![0.0;positions.len()];
        for (i,n) in sorted.iter().enumerate() {
            let x = i as f32;
            for nadj in self.graph.neighbors(n) {
                // calc \delta_x outgoing
                let delta_x = x - node_x[nadj];
                forces[i] = - K * delta_x;
            }
        }*/

        //
        //  b -----> a
        //  1        2
        //  delta_pos = 2 - 1 = 1
        //  F = -k \delta_x =
        //

        // compute forces and update positions


        //self.graph.

        info!("Frame info:");
        info!("Initial:");
        for n in self.graph.node_indices() {
            let task = self.graph.node_weight(n).unwrap();
            info!("  {}(#{})", task.name, n.index());
        }
        info!("Sorted:");
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

