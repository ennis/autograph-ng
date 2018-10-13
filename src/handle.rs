//! Owning handles to vulkan objects.
use std::fmt::Debug;
use std::mem;
use std::ops::Deref;

#[derive(Debug)]
pub struct VkHandle<T: Debug + Clone>(T);

impl<T: Debug + Clone> Deref for VkHandle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Debug + Clone> VkHandle<T> {
    pub fn new(t: T) -> VkHandle<T> {
        VkHandle(t)
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
impl<T: Debug + Clone> Drop for VkHandle<T> {
    fn drop(&mut self) {
        panic!("leaking owned handle")
    }
}
