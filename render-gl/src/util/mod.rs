use std::mem;
use std::sync::Mutex;
use typed_arena::Arena;
use fxhash::FxHashMap;
use std::hash::Hash;
use fxhash::FxBuildHasher;

mod dropless_arena;
pub use self::dropless_arena::DroplessArena;

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

    /// See [typed_arena::Arena].
    pub fn alloc_extend<I>(&self, iterable: I) -> &mut [T]
    where
        I: IntoIterator<Item = T>,
    {
        unsafe {
            mem::transmute::<&mut [T], &mut [T]>(self.0.lock().unwrap().alloc_extend(iterable))
        }
    }

    /*/// See [typed_arena::Arena].
    pub unsafe fn alloc_uninitialized(&self, num: usize) -> *mut [T] {
        self.0.lock().unwrap().alloc_uninitialized(num)
    }*/

    /// See [typed_arena::Arena].
    pub fn into_vec(self) -> Vec<T> {
        self.0.into_inner().unwrap().into_vec()
    }
}

//--------------------------------------------------------------------------------------------------


/// Sync wrapper over a dropless arena.
/// See [typed_arena::Arena].
pub struct SyncDroplessArena(Mutex<DroplessArena>);

impl SyncDroplessArena {
    ///
    #[inline]
    pub fn new() -> Self {
        SyncDroplessArena(Mutex::new(DroplessArena::new()))
    }

    ///
    pub fn alloc<T: Copy>(&self, value: T) -> &mut T {
        // this is (probably) safe because TODO
        unsafe { mem::transmute::<&mut T, &mut T>(self.0.lock().unwrap().alloc(value)) }
    }

    ///
    #[inline]
    pub fn alloc_extend<T: Copy, I>(&self, iterable: I) -> &mut [T]
        where
            I: IntoIterator<Item = T>
    {
        unsafe { mem::transmute::<&mut [T], &mut [T]>(self.0.lock().unwrap().alloc_extend(iterable)) }
    }

    ///
    #[inline]
    pub unsafe fn alloc_uninitialized<T: Copy>(&self, len: usize) -> &mut [T] {
        unsafe { mem::transmute::<&mut [T], &mut [T]>(self.0.lock().unwrap().alloc_uninitialized(len)) }
    }

}

//--------------------------------------------------------------------------------------------------

/// Combination of SyncDroplessArena + HashMap, used for interning stuff.
///
/// Basically an insert-only HashMap which can hand const references to its elements.
pub struct SyncDroplessArenaHashMap<K: Eq + Hash, V: Copy> {
    arena: SyncDroplessArena,
    hash: Mutex<FxHashMap<K, *const V>>,
}

// necessary because of *const V
// TODO audit
unsafe impl<K: Eq + Hash, V: Copy> Sync for SyncDroplessArenaHashMap<K, V> {}

impl<K: Eq + Hash, V: Copy> SyncDroplessArenaHashMap<K, V> {
    pub fn new() -> SyncDroplessArenaHashMap<K, V> {
        SyncDroplessArenaHashMap {
            arena: SyncDroplessArena::new(),
            hash: Mutex::new(FxHashMap::with_hasher(FxBuildHasher::default())),
        }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let mut hash = self.hash.lock().unwrap();
        let arena = &self.arena;
        hash.get(&key).map(|ptr| unsafe { &**ptr })
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
