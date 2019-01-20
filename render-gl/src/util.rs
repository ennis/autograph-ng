use std::mem;
use std::sync::Mutex;
use typed_arena::Arena;

/// Sync wrapper over a typed arena.
/// See [typed_arena::Arena].
pub struct SyncArena<T>(Mutex<Arena<T>>);

impl<T> SyncArena<T> {
    /// See [typed_arena::Arena].
    pub fn new() -> SyncArena<T> {
        SyncArena(Mutex::new(Arena::new()))
    }

    /*/// See [typed_arena::Arena].
    pub fn with_capacity(n: usize) -> SyncArena<T> {
        SyncArena(Mutex::new(Arena::with_capacity(n)))
    }*/

    /// See [typed_arena::Arena].
    pub fn alloc(&self, value: T) -> &mut T {
        // this is (probably) safe because TODO
        unsafe { mem::transmute::<&mut T, &mut T>(self.0.lock().unwrap().alloc(value)) }
    }

    /*/// See [typed_arena::Arena].
    pub fn alloc_extend<I>(&self, iterable: I) -> &mut [T]
    where
        I: IntoIterator<Item = T>,
    {
        unsafe {
            mem::transmute::<&mut [T], &mut [T]>(self.0.lock().unwrap().alloc_extend(iterable))
        }
    }*/

    /*/// See [typed_arena::Arena].
    pub unsafe fn alloc_uninitialized(&self, num: usize) -> *mut [T] {
        self.0.lock().unwrap().alloc_uninitialized(num)
    }*/

    /// See [typed_arena::Arena].
    pub fn into_vec(self) -> Vec<T> {
        self.0.into_inner().unwrap().into_vec()
    }
}
