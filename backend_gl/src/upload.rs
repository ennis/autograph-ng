//! Upload buffers
use crate::buffer::create_buffer;
use crate::{api as gl, api::types::*, api::Gl};
use gfx2::align_offset;
use std::ptr::copy_nonoverlapping;
use std::sync::Mutex;

pub struct MappedBuffer {
    buffer: GLuint,
    ptr: *mut u8,
    size: usize,
    flags: GLenum,
}

unsafe impl Send for MappedBuffer {}

impl MappedBuffer {
    pub fn new(gl: &Gl, size: usize) -> MappedBuffer {
        let buffer = create_buffer(
            gl,
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
            unsafe { gl.MapNamedBufferRange(buffer, 0, size as isize, map_flags) as *mut u8 };

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

    pub fn flush(&self, _gl: &Gl) {
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

    pub fn flush(&self, gl: &Gl) {
        self.0.lock().unwrap().buffer.flush(gl)
    }

    pub fn into_inner(self) -> MappedBuffer {
        self.0.into_inner().unwrap().buffer
    }
}
