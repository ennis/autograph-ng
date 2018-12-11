use std::mem;
use std::ptr;

use crate::renderer::backend::gl::api as gl;
use crate::renderer::backend::gl::api::types::*;

//--------------------------------------------------------------------------------------------------

/// Copy + Clone to bypass a restriction of slotmap on stable rust.
#[derive(Copy, Clone, Debug)]
pub struct RawBuffer {
    pub obj: GLuint,
    pub size: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct BufferDescription {
    pub size: usize,
}

//--------------------------------------------------------------------------------------------------
pub fn create_buffer(byte_size: usize, flags: GLenum, initial_data: Option<&[u8]>) -> GLuint {
    let mut obj: GLuint = 0;
    unsafe {
        gl::CreateBuffers(1, &mut obj);
        gl::NamedBufferStorage(
            obj,
            byte_size as isize,
            if let Some(data) = initial_data {
                data.as_ptr() as *const GLvoid
            } else {
                ptr::null()
            },
            flags,
        );
    }

    obj
}

/*
/// Trait for a thing that provides vertex data
pub trait VertexDataSource {
type ElementType: VertexType;
}

impl<T: VertexType> VertexDataSource for Buffer<[T]> {
type ElementType = T;
}

impl<T: VertexType> VertexDataSource for BufferSlice<[T]> {
type ElementType = T;
}

/// Trait for a thing that provides index data
pub trait IndexDataSource {
type ElementType: IndexElementType;
}

impl<T: IndexElementType> IndexDataSource for Buffer<[T]> {
type ElementType = T;
}

impl<T: IndexElementType> IndexDataSource for BufferSlice<[T]> {
type ElementType = T;
}

/*pub trait AsSlice<T: BufferData + ?Sized> {
    fn as_slice(&self) -> BufferSlice<T>;
    fn as_slice_any(&self) -> BufferSliceAny;
    unsafe fn get_slice_any(&self, byte_offset: usize, byte_size: usize) -> BufferSliceAny;
}

impl<T: BufferData + ?Sized> AsSlice<T> for Arc<Buffer<T>> {
    fn as_slice(&self) -> BufferSlice<T> {
        BufferSlice {
            owner: self.clone(),
            len: self.len,
            byte_offset: 0,
            _phantom: PhantomData,
        }
    }

    // Type-erased version of the above
    fn as_slice_any(&self) -> BufferSliceAny {
        BufferSliceAny {
            owner: self.clone(),
            byte_size: self.byte_size(),
            byte_offset: 0,
        }
    }

    unsafe fn get_slice_any(&self, byte_offset: usize, byte_size: usize) -> BufferSliceAny {
        // TODO check that the range is inside
        BufferSliceAny {
            owner: self.clone(),
            byte_size: byte_size,
            byte_offset: byte_offset,
        }
    }
}*/
impl Drop for RawBufferObject {
fn drop(&mut self) {
unsafe {
gl::DeleteBuffers(1, &self.obj);
}
}
}
*/
