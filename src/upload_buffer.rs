use super::buffer::{BufferStorage, BufferUsage, BufferSlice};
use super::buffer_data::BufferData;
use super::context::{Context, FrameNumber};
use std::cell::RefCell;
use std::collections::vec_deque::VecDeque;
use std::mem;
use std::ptr::copy_nonoverlapping;

struct Region {
    frame_number: FrameNumber,
    begin_ptr: usize,
    end_ptr: usize,
}

pub struct UploadBufferState {
    write: usize,
    begin: usize,
    used: usize,
    regions: VecDeque<Region>,
}

/// A ring buffer for uploading data to GPU memory.
/// Each upload is put in a memory region, and has an associated frame number.
/// Before each upload, 'old' regions with a frame_number lower than or equal to a certain threshold are reclaimed
/// and reused for subsequent uploads.
/// Typically, this threshold is current_frame_number - max_number_of_pipelined_frames.
pub(crate) struct UploadBuffer {
    pub(crate) buffer: BufferStorage,
    state: RefCell<UploadBufferState>,
    pub(crate) mapped_region: *mut u8,
}

fn align_offset(align: usize, size: usize, ptr: usize, space: usize) -> Option<usize> {
    let mut off = ptr & (align - 1);
    if off > 0 {
        off = align - off;
    };
    if space < off || space - off < size {
        None
    } else {
        Some(ptr + off)
    }
}

impl UploadBuffer {
    /// Creates a new upload buffer. The buffer must have been created with the 'Upload' usage.
    /// Takes ownership of the buffer.
    /// TODO should we pass the buffer as an argument or should the upload buffer create and own
    /// its own buffer?
    pub(crate) fn new(size: usize) -> UploadBuffer {
        //assert_eq!(buffer.usage, BufferUsage::Upload, "Buffer must be created with `Upload` usage");
        let buffer = BufferStorage::new(size, BufferUsage::Upload);
        // Map buffer memory.
        // FIXME This assumes that the buffer has been allocated with the PERSISTENT and COHERENT bits (which should be the case with the Upload mode)
        let mapped_region = unimplemented!();
        /*unsafe { buffer.map_all(
            gl::MAP_UNSYNCHRONIZED_BIT
                | gl::MAP_WRITE_BIT
                | gl::MAP_PERSISTENT_BIT
                | gl::MAP_COHERENT_BIT) as *mut u8 };*/

        UploadBuffer {
            buffer,
            state: RefCell::new(UploadBufferState {
                begin: 0,
                used: 0,
                write: 0,
                regions: VecDeque::new(),
            }),
            mapped_region,
        }
    }

    /// Destroys this upload buffer and returns the buffer storage.
    pub(crate) fn destroy(self) -> BufferStorage {
        self.buffer
    }

    /// Uploads data to this upload buffer.
    /// All regions with frame_number <= reclaim_until are reclaimed.
    pub(crate) fn upload<T: BufferData + ?Sized>(
        &self,
        data: &T,
        align: usize,
        frame_number: FrameNumber,
        reclaim_until: FrameNumber,
    ) -> BufferSlice
    {
        let byte_size = mem::size_of_val(data);
        let ptr = data as *const T as *const u8;
        let slice = self
            .allocate(byte_size, align, frame_number, reclaim_until)
            .expect("upload buffer is full"); // TODO expand? wait? return None?
        unsafe {
            // This is safe because `allocate` makes sure that the target memory region at mapped_region+offset
            // is sufficiently large to accomodate the data.
            // Also, the data we copy is 'static and does not contain any references.
            // FIXME we assume that the reference to 'data' does not point to a memory region
            // FIXME inside the mapped buffer: is that okay? the client should have no way to create
            // FIXME a pointer inside this memory region.
            copy_nonoverlapping(
                ptr,
                self.mapped_region.offset(slice.offset as isize),
                byte_size,
            );
        }
        slice
    }

    fn allocate(
        &self,
        size: usize,
        align: usize,
        frame_number: FrameNumber,
        reclaim_until: FrameNumber,
    ) -> Option<BufferSlice> {
        //debug!("alloc size={}, align={}, fence_value={:?}", size, align, fence_value);
        if let Some(offset) = self.try_allocate_contiguous(size, align, frame_number) {
            Some(BufferSlice {offset, size})
        } else {
            // reclaim and try again (not enough contiguous free space)
            // FIXME unconditionally reclaim?
            self.reclaim(reclaim_until);
            if let Some(offset) = self.try_allocate_contiguous(size, align, frame_number) {
                Some(BufferSlice {offset, size})
            } else {
                None
            }
        }
    }

    fn try_allocate_contiguous(
        &self,
        size: usize,
        align: usize,
        frame_number: FrameNumber,
    ) -> Option<usize> {
        //assert!(size < self.buffer.size);
        let mut state = self.state.borrow_mut();

        if (state.begin < state.write) || (state.begin == state.write && state.used == 0) {
            let slack = self.buffer.size - state.write;
            // try to put the buffer in the slack space at the end
            if let Some(newptr) = align_offset(align, size, state.write, slack) {
                state.write = newptr;
            } else {
                // else, try to put it at the beginning (which is always correctly
                // aligned)
                if size > state.begin {
                    return None;
                }
                state.write = 0;
            }
        } else {
            // begin_ptr > write_ptr
            // reclaim space in the middle
            if let Some(newptr) = align_offset(align, size, state.write, state.begin - state.write) {
                state.write = newptr;
            } else {
                return None;
            }
        }

        let alloc_begin = state.write;
        state.used += size;
        state.write += size;
        state.regions.push_back(Region {
            begin_ptr: alloc_begin,
            end_ptr: alloc_begin + size,
            frame_number,
        });
        Some(alloc_begin)
    }

    fn reclaim(&self, reclaim_until: FrameNumber) {
        //debug!("reclaiming: last_completed_fence_step={:?}", last_completed_fence_step);
        let mut state = self.state.borrow_mut();
        while !state.regions.is_empty()
            && state.regions.front().unwrap().frame_number <= reclaim_until
            {
                let region = state.regions.pop_front().unwrap();
                //debug!("reclaiming region {}-{} because all commands using these regions have completed (region={:?} < last_completed_fence_step={:?})", region.begin_ptr, region.end_ptr, region.fence_value, last_completed_fence_step);
                state.begin = region.end_ptr;
                state.used -= region.end_ptr - region.begin_ptr;
            }
    }
}
