

extern crate petgraph;
#[macro_use] extern crate bitflags;

mod texture;
mod format;
mod buffer;

use std::rc::Rc;
use texture::TextureDesc;
use buffer::BufferDesc;

type TextureStorage = u16;
type BufferStorage = u16;

/// Represents a texture resource.
struct Texture(Rc<TextureStorage>, TextureDesc);

/// Represents a buffer resource.
/// The actual storage object for the buffer is abstracted away.
struct Buffer(Rc<BufferStorage>, BufferDesc);

/// Represents an operation in the frame graph.
struct Node
{
}

///
enum ResourceInfo
{
    Texture(TextureDesc),
    Buffer(BufferDesc)
}

/// Main graphics context.
/// Handles allocation of persistent resources.
struct Context {}

impl Context {
    /// Creates a new context
    fn new() -> Context {
        unimplemented!()
    }

    /// Returns information about a texture resource from an ID.
    fn get_resource_info(rref: &ResourceRef) -> ResourceInfo {
        unimplemented!()
    }
}

/// A frame: manages transient resources within and across frames.
struct Frame {}

impl Frame {
    /// Returns information about a resource (Transient or Persistent)
    fn get_resource_info(&self, rref: &ResourceRef) -> ResourceInfo {
        unimplemented!()
    }

    /// Creates a write-dependency between the specified node and resource.
    /// Returns a reference to the new version of the resource.
    fn write_dependency(&self, node: NodeID, resource: ResourceRef) -> ResourceRef
    {
        unimplemented!()
    }

    /// Creates a read-dependency between the specified node and resource.
    fn read_dependency(&self, node: NodeID, resource: ResourceRef)
    {
        unimplemented!()
    }

    /// Creates a transient texture and returns a reference to it.
    fn create_transient_texture(&self) -> ResourceRef {
        unimplemented!()
    }
}

/// Represents a reference to a resource: either a persistent resource, or
/// a transient inside a frame.
/// Crucially, this isn't clone: the write_xxx methods in frame take transient handles by value
/// to prevent concurrent write accesses.
enum ResourceRef
{
    /// Persistent resource, with ID and revision index
    Persistent(usize, u16),
    /// Transient resource, with frame-local ID
    Transient(usize)
}

enum DepKind
{
    Read,
    Write
}

/// Represents a dependency between nodes in the frame graph.
struct Dependency
{
    rref: ResourceRef,
    kind: DepKind
}


fn main() {
    let mut context = Context::new();
}
