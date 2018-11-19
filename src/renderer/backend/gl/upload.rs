//! Upload buffers (frame-synchronized ring buffers)
use std::cell::RefCell;
use std::collections::vec_deque::VecDeque;
use std::mem;
use std::ops::Range;
use std::ptr;
use std::ptr::copy_nonoverlapping;

use crate::renderer::backend::gl::api as gl;
use crate::renderer::backend::gl::api::types::*;
use crate::renderer::backend::gl::buffer::create_buffer;
use crate::renderer::backend::gl::sync::{Timeline, Timeout};
use crate::renderer::util::align_offset;

/*
struct FencedRegion {
    fence_value: FenceValue,
    begin_ptr: usize,
    end_ptr: usize,
}

pub struct UploadBufferState {
    write: usize,
    begin: usize,
    used: usize,

    fenced_regions: VecDeque<FencedRegion>,
    // TODO frame fences
    //frame_fences:
}
*/
const MULTI_BUFFER_COUNT: usize = 3;

pub struct MappedBufferRange {
    buffer: GLuint,
    ptr: *mut u8,
    size: usize,
    flags: GLenum,
}

unsafe impl Send for MappedBufferRange {}

impl MappedBufferRange {
    pub fn write(&self, data: &[u8], offset: usize) {
        unsafe {
            copy_nonoverlapping(data.as_ptr(), self.ptr.offset(offset as isize), data.len());
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
}

/// Only supports upload.
pub struct MultiBuffer {
    buffer: GLuint,
    frames: Vec<u64>,
    cur_idx: usize,
    mapped: *mut u8,
    size: usize,
}

unsafe impl Send for MultiBuffer {}

impl MultiBuffer {
    pub fn new(size: usize) -> MultiBuffer {
        // TODO align size properly
        let total_size = MULTI_BUFFER_COUNT * size;
        let buffer = create_buffer(
            total_size,
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
            None,
        );
        // map the buffer
        let mapped = unsafe {
            let map_flags = gl::MAP_UNSYNCHRONIZED_BIT
                | gl::MAP_WRITE_BIT
                | gl::MAP_PERSISTENT_BIT
                | gl::MAP_COHERENT_BIT;
            gl::MapNamedBufferRange(buffer, 0, total_size as isize, map_flags) as *mut u8
        };

        MultiBuffer {
            buffer,
            mapped,
            frames: vec![0; MULTI_BUFFER_COUNT],
            cur_idx: 0,
            size,
        }
    }

    /// Once finished writing to the mapped memory, flush it.
    pub fn acquire_buffer_range(
        &mut self,
        frame_number: u64,
        frame_timeline: &mut Timeline,
    ) -> MappedBufferRange {
        let i = self.cur_idx;
        frame_timeline.client_sync(self.frames[i], Timeout::Infinite);
        self.frames[i] = frame_number;

        self.cur_idx = (self.cur_idx + 1) % MULTI_BUFFER_COUNT;

        MappedBufferRange {
            buffer: self.buffer,
            ptr: unsafe { self.mapped.offset((i * self.size) as isize) },
            flags: gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
            size: self.size,
        }
    }
}

pub struct MappedBufferRangeStack {
    mapped: MappedBufferRange,
    offset: usize,
}

impl MappedBufferRangeStack {
    pub fn new(mapped: MappedBufferRange) -> MappedBufferRangeStack {
        MappedBufferRangeStack { mapped, offset: 0 }
    }

    /// Panics if not enough space available. Returns the offset.
    pub fn write(&mut self, data: &[u8], align: usize) -> Option<usize> {
        let offset = align_offset(
            data.len() as u64,
            align as u64,
            (self.offset as u64)..(self.mapped.size as u64),
        )? as usize;
        let slice = self.mapped.write(data, offset);
        self.offset = offset + data.len();
        Some(offset)
    }

    pub fn flush(&self) {
        self.mapped.flush()
    }
}
