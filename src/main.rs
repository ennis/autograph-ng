

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

/// Represents a persistent texture resource.
struct Texture(Rc<TextureStorage>, TextureDesc);

/// Represents a persistent buffer resource.
/// The actual storage object for the buffer is abstracted away.
struct Buffer(Rc<BufferStorage>, BufferDesc);

/// Represents an operation in the frame graph.
struct Node
{
}

/// Represents a reference to a resource: either a persistent resource, or
/// a transient inside a frame.
enum ResourceRef
{
    Persistent(),
    Transient()
}

enum DepKind
{
    Read,
    Write
}

/// Represents a dependency between nodes in the frame graph.
struct Dependency
{
    resource_ref: ResourceRef,
    kind: DepKind
}

/// A handle to a resource: this is manipulated by the user.
/// Crucially, this isn't clone: the write_xxx methods in frame take handles by value
/// to prevent concurrent write accesses.
trait ResourceHandle
{
    //type
}

// TextureHandle, Texture2DHandle, BufferHandle
//  get_desc() -> TextureXXX

/// The context: handles allocation of persistent resources.
struct Context {}
struct Frame {}

fn main() {
    println!("Hello, world!");
}
