
use petgraph::{Graph, Direction, Directed, graph::NodeIndex};

/// Represents an operation in the frame graph.
struct Node
{
}

/// Represents a task node in the frame graph.
pub(crate) struct Task {}

#[derive(Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub struct ResourceId(pub(crate) u32);
type TaskIndex = u32;
pub(crate) type TaskId = NodeIndex<TaskIndex>;
type FrameGraph = Graph<Task, Dependency, Directed, TaskId>;

/// How a node accesses a resource.
enum ResourceAccess
{
    /// Node reads the resource.
    Read,
    /// Node modifies the resource.
    Modify
}

/// Represents a dependency between nodes in the frame graph.
struct Dependency
{
    //rref: ResourceRef,
    access: ResourceAccess
}

/*
/// A transient resource that lives only during one frame and is reclaimed after.
/// The attached storage can be aliased between resources if the system determines that
/// no overlap is possible.
pub(crate) enum TransientResource
{
    Texture(Texture),
    Buffer(Buffer),
}

impl TransientResource
{
}

/// A frame: manages transient resources within and across frames.
pub struct Frame<'ctx> {
    context: &'ctx mut Context,
    /// The DAG of tasks.
    graph: FrameGraph,
    /// Table of transient resources for this frame.
    resources: Vec<TransientResource>,
    /// The root node from which all transient resources originates.
    /// This is just here to avoid an Option<> into ResourceRefs
    transient_root: TaskID,
}

impl<'ctx> Frame<'ctx> {
    /// Creates a new frame.
    fn new(context: &'ctx mut Context) -> Frame<'ctx> {
        let mut graph = FrameGraph::new();
        // create a dummy task for the transient root.
        let transient_root = graph.add_node(Task {});
        let mut f = Frame {
            graph,
            context,
            resources: Vec::new(),
            transient_root
        };
    }

    /// Creates a new task.
    /// Returns the ID to the newly created task.
    fn create_task(&mut self) -> TaskID {
        self.graph.add_node(Task {})
    }

    /// Updates the data contained in a texture. This creates a task in the graph.
    /// This does not synchronize: the data to be modified is first uploaded into a
    /// staging buffer first.
    fn update_texture(&mut self, tex: ResourceRef, data: ()) -> ResourceRef {
        unimplemented!()
    }

    /// Returns information about a texture (Transient or Persistent)
    /// Panics if the reference does not point to a texture.
    fn get_texture_desc(&self, rref: ResourceRef) -> TextureDesc {
        unimplemented!()
    }

    /// Returns information about a buffer (Transient or Persistent)
    /// Panics if the reference does not point to a buffer.
    fn get_buffer_desc(&self, rref: ResourceRef) -> BufferDesc {
        unimplemented!()
    }

    /// Creates a write-dependency between the specified node and resource.
    /// Returns a reference to the new version of the resource.
    fn write_dependency(&mut self, task: TaskID, resource: ResourceRef) -> ResourceRef
    {
        //self.graph.add_edge()
        unimplemented!()
    }

    /// Creates a read-dependency between the specified node and resource.
    fn read_dependency(&mut self, task: TaskID, resource: ResourceRef)
    {
        unimplemented!()
    }

    /// Creates a transient texture and returns a reference to it.
    fn create_transient_texture(&mut self, desc: &TextureDesc) -> ResourceRef {
        let id = ResourceID(self.resources.len() as u32);
        self.resources.push(TransientResource::Texture(unimplemented!()));
        ResourceRef::Transient {
            id,
            task: self.transient_root,
        }
    }

    /// Creates a transient buffer and returns a reference to it.
    fn create_transient_buffer(&mut self, desc: &BufferDesc) -> ResourceRef {
        unimplemented!()
    }
}

*/