/*
/// Conceptually, this borrows the buffer. However, actual borrowing gets rather inconvenient at times
/// mostly due to the inability to store borrower and borrowed in the same struct.
/// So instead do stuff dynamically.
#[derive(Debug)]
pub struct MappedBufferRange<R: RendererBackend> {
    buffer: R::BufferHandle,
    ptr: *mut u8,
    size: usize,
    //flags: GLenum,
}

unsafe impl<R: RendererBackend> Send for MappedBufferRange<R> {}

impl<R: RendererBackend> MappedBufferRange<R> {

    /// (Should) Panic if the underlying buffer has been deleted.
    pub fn write(&self, data: &[u8], offset: usize) {
        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(offset), data.len());
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

    pub fn buffer(&self) -> GLuint {
        self.buffer
    }
}

// rust issue #26925
impl<R: RendererBackend> Clone for MappedBufferRange<R> {
    fn clone(&self) -> Self {
        // safe because TODO
        // has no dynamically allocated components or mutable refs
        unsafe { mem::transmute_copy(self) }
    }
}*/
