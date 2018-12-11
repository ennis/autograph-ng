//! Upload buffers
use std::mem;
use std::ops::Range;
use std::ptr;
use std::ptr::copy_nonoverlapping;
use std::sync::Mutex;

use crate::renderer::backend::gl::api as gl;
use crate::renderer::backend::gl::api::types::*;
use crate::renderer::backend::gl::buffer::create_buffer;
use crate::renderer::backend::gl::sync::{Timeline, Timeout};
use crate::renderer::util::align_offset;

pub struct MappedBuffer {
    buffer: GLuint,
    ptr: *mut u8,
    size: usize,
    flags: GLenum,
}

unsafe impl Send for MappedBuffer {}

impl MappedBuffer {
    pub fn new(size: usize) -> MappedBuffer {
        let buffer = create_buffer(
            size,
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
            None,
        );
        // map the buffer
        let map_flags = gl::MAP_UNSYNCHRONIZED_BIT
            | gl::MAP_WRITE_BIT
            | gl::MAP_PERSISTENT_BIT
            | gl::MAP_COHERENT_BIT;
        let ptr =
            unsafe { gl::MapNamedBufferRange(buffer, 0, size as isize, map_flags) as *mut u8 };

        MappedBuffer {
            buffer,
            ptr,
            size,
            flags: map_flags,
        }
    }

    pub fn write(&self, data: &[u8], offset: usize) {
        unsafe {
            copy_nonoverlapping(data.as_ptr(), self.ptr.add(offset), data.len());
        }
    }

    pub fn flush(&self) {
        if (self.flags & gl::MAP_COHERENT_BIT) != 0 {
            // do nothing, data is already visible to the CPU
        } else {
            // TODO glFlushMappedBufferRange
            unimplemented!()
        }
    }

    pub fn raw_buffer(&self) -> GLuint {
        self.buffer
    }
}

struct UploadBufferInner {
    buffer: MappedBuffer,
    offset: usize,
}

pub struct UploadBuffer(Mutex<UploadBufferInner>);

impl UploadBuffer {
    pub fn new(buffer: MappedBuffer) -> UploadBuffer {
        UploadBuffer(Mutex::new(UploadBufferInner { buffer, offset: 0 }))
    }

    /// Returns the offset.
    pub fn write(&self, data: &[u8], align: usize) -> Option<(GLuint, usize)> {
        let mut self_ = self.0.lock().unwrap();

        let offset = align_offset(
            data.len() as u64,
            align as u64,
            (self_.offset as u64)..(self_.buffer.size as u64),
        )? as usize;
        self_.buffer.write(data, offset);
        self_.offset = offset + data.len();
        Some((self_.buffer.raw_buffer(), offset))
    }

    pub fn flush(&self) {
        self.0.lock().unwrap().buffer.flush()
    }

    pub fn into_inner(self) -> MappedBuffer {
        self.0.into_inner().unwrap().buffer
    }
}
