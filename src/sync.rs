//! Frame-based synchronization primitives.
//!
//! Some objects are attached to a frame (or sequence of frames),
//! and should not be deleted until those frames are deleted.

use ash::vk;

use context::{VkDevice1, FrameNumber};

/// An object (or group of objects) that is bound to a particular frame:
/// i.e. that should not be deleted until the frame is retired.
pub(crate) struct FrameBoundObject<T: ?Sized>
{
    frame_number: FrameNumber,
    obj: T
}

/// An object used to wait for stuff
struct FrameSync
{
    current_frame: FrameNumber,
    last_retired_frame: FrameNumber,
    fences: Vec<Vec<vk::Fence>>,
}

impl<T: ?Sized> FrameBoundObject<T>
{
    ///
    pub(crate) fn try_delete(self, vkd: &VkDevice1, frame_sync: &mut FrameSync, deleter: impl FnOnce(T)) -> Option<Self>
    {
        // wait for the frame to be completely finished
        let wait_result = frame_sync.try_wait_complete(self.frame_number);
        match wait_result {
            Ok(_) => {
                deleter(self.obj);
                None
            },
            Err(_) => self
        }
    }

    ///
    pub(crate) fn wait_delete(self, vkd: &VkDevice1, frame_sync: &mut FrameSync, deleter: impl FnOnce(T))
    {
        frame_sync.wait_complete(self.frame_number).expect("failure waiting for frame to complete");
        deleter(self.obj);
    }
}

struct WaitList<T>
{
    objects: Vec<FrameBoundObject<T>>,
}


impl FrameSync
{

}

impl<T> WaitList<T>
{
    pub(crate) fn enqueue(&mut self, obj: T, current_frame: FrameNumber, last_retired_frame: FrameNumber) {
        unimplemented!()
    }

    pub(crate) fn reclaim(&mut self, last_retired_frame: FrameNumber, reclaimer: impl FnMut(T)) {
        unimplemented!()
    }
}

/*impl WaitList
{
    /// Will panic if
    pub(crate) fn enqueue(&self, frame_fence: )
}*/