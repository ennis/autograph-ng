//! Frame-based synchronization primitives.

use ash::vk;

use context::FrameNumber;

/// An object (or group of objects) that is bound to a particular frame:
/// i.e. that should not be deleted until the frame is retired.
pub(crate) struct FrameBoundObject<T: ?Sized>
{
    frame_number: FrameNumber,
    obj: T
}

/// An object used to wait for
struct FrameSync<'f>
{
    current_frame: FrameNumber,
    last_retired_frame: FrameNumber,
    fences: &'f [vk::Fence],
}

impl<T: ?Sized> FrameBoundObject<T>
{
    pub(crate) fn wait()
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