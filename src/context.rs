use upload_buffer::UploadBuffer;
use buffer::{BufferSlice, BufferDesc, BufferStorage};
use texture::{TextureDesc, TextureObject};
use frame::{TaskId, ResourceId};
use std::rc::Rc;

pub type FrameNumber = usize;
pub type ResourceInfo = ();

/// Represents a texture resource.
pub struct Texture {
    /// The storage for this texture (the actual GL object).
    pub(crate) storage: Rc<TextureObject>,
    /// Does this resource only live for the current frame?
    pub(crate) transient: Option<FrameNumber>
}

/// Represents a buffer resource.
/// The actual storage object for the buffer is abstracted away.
pub struct Buffer {
    /// The OpenGL buffer object that backs this buffer.
    /// This can be shared by multiple buffers.
    pub(crate) storage: Rc<BufferStorage>,
    /// Offset and size of the buffer inside the storage.
    pub(crate) slice: BufferSlice,
    /// Does this resource only live for the current frame?
    pub(crate) transient: Option<FrameNumber>
}

/// Represents a reference to a resource: either a persistent resource, or
/// a transient inside a frame.
#[derive(Copy,Clone,Debug)]
pub struct ResourceRef
{
    /// Index of the resource in the table.
    id: ResourceId,
    /// Task that generated this revision of the resource.
    task: TaskId,
}


/// Main graphics context.
/// Handles allocation of persistent resources.
pub struct Context {
    ///// The main upload buffer, where transient resources such as dynamic uniform buffers are allocated.
    //pub(crate) upload_buffer: UploadBuffer,
}

impl Context {
    /// Creates a new context
    pub fn new() -> Context {
        Context {
            // 3 MiB upload buffer
            // TODO configurable size?
            //upload_buffer: UploadBuffer::new(3*1024*1024)
        }
    }

    /// Returns information about a texture resource from an ID.
    pub fn get_resource_info(&self, resource: &ResourceRef) -> ResourceInfo {
        unimplemented!()
    }

    /// Creates a persistent texture, with optional initial data.
    pub fn create_texture(&mut self, desc: &TextureDesc) -> Texture {
        unimplemented!()
    }

    /// Creates a persistent buffer.
    pub fn create_buffer(&mut self, desc: &BufferDesc) -> Buffer {
        unimplemented!()
    }

    /*/// Creates a frame.
    pub fn new_frame<'a>(&'a mut self) -> Frame<'a> {
        unimplemented!()
    }*/
}
