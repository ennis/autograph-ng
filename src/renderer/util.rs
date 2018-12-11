use std::mem;
use std::ops::Range;
use std::sync::Mutex;
use typed_arena::Arena;

pub fn align_offset(size: u64, align: u64, space: Range<u64>) -> Option<u64> {
    assert!(align.is_power_of_two(), "alignment must be a power of two");
    let mut off = space.start & (align - 1);
    if off > 0 {
        off = align - off;
    }
    if space.start + off + size > space.end {
        None
    } else {
        Some(space.start + off)
    }
}

/// Sync wrapper over typed arena
pub struct SyncArena<T>(Mutex<Arena<T>>);

impl<T> SyncArena<T> {
    pub fn new() -> SyncArena<T> {
        SyncArena(Mutex::new(Arena::new()))
    }

    pub fn with_capacity(n: usize) -> SyncArena<T> {
        SyncArena(Mutex::new(Arena::with_capacity(n)))
    }

    pub fn alloc(&self, value: T) -> &mut T {
        // this is (probably) safe because TODO
        unsafe { mem::transmute::<&mut T, &mut T>(self.0.lock().unwrap().alloc(value)) }
    }

    pub fn alloc_extend<I>(&self, iterable: I) -> &mut [T]
    where
        I: IntoIterator<Item = T>,
    {
        unsafe {
            mem::transmute::<&mut [T], &mut [T]>(self.0.lock().unwrap().alloc_extend(iterable))
        }
    }

    pub unsafe fn alloc_uninitialized(&self, num: usize) -> *mut [T] {
        self.0.lock().unwrap().alloc_uninitialized(num)
    }

    pub fn into_vec(self) -> Vec<T> {
        self.0.into_inner().unwrap().into_vec()
    }
}
