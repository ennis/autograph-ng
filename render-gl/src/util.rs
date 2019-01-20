use std::mem;
use std::sync::Mutex;
use typed_arena::Arena;
use fxhash::FxHashMap;
use std::hash::Hash;
use fxhash::FxBuildHasher;

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

/// Combination of SyncArena + HashMap, used for interning stuff.
///
/// Basically an insert-only HashMap which can hand const references to its elements.
pub struct SyncArenaHashMap<K: Eq + Hash, V> {
    arena: SyncArena<V>,
    hash: Mutex<FxHashMap<K,*const V>>
}

// necessary because of *const V
// TODO audit
unsafe impl<K: Eq + Hash, V> Sync for SyncArenaHashMap<K, V>
{}

impl<K: Eq + Hash, V> SyncArenaHashMap<K,V> {
    pub fn new() -> SyncArenaHashMap<K,V> {
        SyncArenaHashMap {
            arena: SyncArena::new(),
            hash: Mutex::new(FxHashMap::with_hasher(FxBuildHasher::default()))
        }
    }

    pub fn get_or_insert_with(&self, key: K, f: impl FnOnce() -> V) -> &V {
        let mut hash = self.hash.lock().unwrap();
        let arena = &self.arena;
        let ptr = *hash.entry(key).or_insert_with(|| {
            let ptr = arena.alloc(f());
            ptr as *const _
        });

        // safe because:
        // - no mutable borrows exist
        // - the data pointed to never moves
        // TODO probably more details about safety to figure out
        unsafe {
            // reborrow as ref
            &*ptr
        }
    }
}

