use frame::dependency::Dependency;
use frame::tasks::Task;

use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    visit::EdgeRef,
    Directed, Direction, Graph,
};

//--------------------------------------------------------------------------------------------------
// Frame graph
pub type TaskId = NodeIndex<u32>;
pub type DependencyId = EdgeIndex<u32>;

/// The frame graph type.
pub type FrameGraph = Graph<Box<Task>, Dependency, Directed, u32>;
