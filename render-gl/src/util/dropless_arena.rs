use std::cell::RefCell;
use std::cmp;
use std::cmp::max;
use std::iter;
use std::mem;
use std::ptr;
use std::slice;

// Initial size in bytes.
const INITIAL_SIZE: usize = 1024;
// Minimum capacity. Must be larger than 0.
const MIN_CAPACITY: usize = 1;

struct ChunkList<T> {
    current: Vec<T>,
    rest: Vec<Vec<T>>,
}

impl<T> ChunkList<T> {
    #[inline(never)]
    #[cold]
    fn reserve(&mut self, additional: usize) {
        let double_cap = self
            .current
            .capacity()
            .checked_mul(2)
            .expect("capacity overflow");
        let required_cap = additional
            .checked_next_power_of_two()
            .expect("capacity overflow");
        let new_capacity = cmp::max(double_cap, required_cap);
        let chunk = mem::replace(&mut self.current, Vec::with_capacity(new_capacity));
        self.rest.push(chunk);
    }
}

/// An arena for Copy types (which don't have any destructors, hence the name).
pub struct DroplessArena {
    chunks: RefCell<ChunkList<u8>>,
}

impl DroplessArena {
    /// Creates a new arena with an unspecified default initial capacity.
    pub fn new() -> DroplessArena {
        DroplessArena::with_capacity(INITIAL_SIZE)
    }

    /// Creates a new arena with a specified initial capacity.
    pub fn with_capacity(n: usize) -> DroplessArena {
        let n = cmp::max(MIN_CAPACITY, n);
        DroplessArena {
            chunks: RefCell::new(ChunkList {
                current: Vec::with_capacity(n),
                rest: Vec::new(),
            }),
        }
    }

    /// Stores a value of type `T` into the arena.
    ///
    /// Returns a mutable reference to the stored values.
    pub fn alloc<T: Copy>(&self, value: T) -> &mut T {
        &mut self.alloc_extend(iter::once(value))[0]
    }

    /// Allocates uninitialized space for `len` elements of type `T`.
    pub unsafe fn alloc_uninitialized<T: Copy>(&self, len: usize) -> &mut [T] {
        self.alloc_extend(iter::repeat(mem::uninitialized()).take(len))
    }

    /// Stores all elements from the provided iterator into a contiguous slice inside the arena.
    ///
    /// Returns a mutable slice to the stored values.
    ///
    /// # Note
    /// This method uses `Iterator::size_hint` to preallocate space in the arena.
    /// This is more efficient if the exact number of elements returned by the iterator is known in
    /// advance.
    pub fn alloc_extend<I, T: Copy>(&self, iterable: I) -> &mut [T]
    where
        I: IntoIterator<Item = T>,
    {
        let itemsize = mem::size_of::<T>();
        let itemalign = mem::align_of::<T>();

        let mut iter = iterable.into_iter();
        let mut chunks = self.chunks.borrow_mut();

        let iter_min_len = iter.size_hint().0 * itemsize;

        let mut i = 0;
        let mut cur_len = {
            let len = chunks.current.len();
            let align_offset =
                unsafe { chunks.current.as_mut_ptr().add(len) }.align_offset(itemalign);
            len + align_offset
        };
        let mut start = cur_len;

        while let Some(elem) = iter.next() {
            let cap = chunks.current.capacity();

            if cur_len + max(itemsize, iter_min_len) > cap {
                // The iterator was larger than we could fit into the current chunk.
                let chunks = &mut *chunks;
                // Create a new chunk into which we can freely push the entire iterator into
                // i + 1 for the next one, and * 2 to be sure (and have enough alignment space)
                let newchunksize = max((i + 1) * 2 * itemsize, iter_min_len + itemsize);
                chunks.reserve(newchunksize);
                let previous_chunk = chunks.rest.last_mut().unwrap();

                //let previous_chunk_len = previous_chunk.len();
                unsafe {
                    // copy data from previous chunk
                    let ptr = chunks.current.as_mut_ptr();
                    let align_offset = ptr.align_offset(itemalign);
                    let dest = ptr.add(align_offset) as *mut T;
                    let src = previous_chunk.as_ptr().add(start) as *const T;
                    ptr::copy_nonoverlapping(src, dest, i);
                    cur_len = align_offset + i * itemsize;
                    // adjust lengths
                    chunks.current.set_len(cur_len);
                    //previous_chunk.set_len(start);  // unnecessary
                    start = align_offset;
                }
            }

            // insert element (enough size now)
            unsafe {
                let ptr = chunks.current.as_mut_ptr().add(cur_len) as *mut T;
                ptr.write(elem);
                cur_len += itemsize;
                chunks.current.set_len(cur_len);
            }

            i += 1;
        }

        let new_slice = unsafe {
            let new_slice =
                slice::from_raw_parts_mut(chunks.current.as_mut_ptr().add(start) as *mut T, i);
            mem::transmute::<&mut [T], &mut [T]>(new_slice)
        };

        new_slice
    }
}
