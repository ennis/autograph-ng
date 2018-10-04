//! Owning handles to vulkan objects.
use std::fmt::Debug;
use std::mem;
use std::ops::Deref;

#[derive(Debug)]
pub struct OwnedHandle<T: Debug + Clone>(T);

impl<T: Debug + Clone> Deref for OwnedHandle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Debug + Clone> OwnedHandle<T> {
    pub fn new(t: T) -> OwnedHandle<T> {
        OwnedHandle(t)
    }

    pub fn get(&self) -> T {
        self.0.clone()
    }

    pub fn destroy(mut self, deleter: impl FnOnce(T)) {
        let inner = unsafe { mem::replace(&mut self.0, mem::uninitialized()) };
        deleter(inner);
        mem::forget(self)
    }
}

// Drop bomb
impl<T: Debug + Clone> Drop for OwnedHandle<T> {
    fn drop(&mut self) {
        panic!("leaking owned handle")
    }
}
