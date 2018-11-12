use ash::vk;

use crate::device::{Device, FrameNumber};

pub trait DeviceBoundObject {
    fn device(&self) -> &Device;
}

pub unsafe trait FrameSynchronizedObject {
    type Proxy;

    /// Should be called when the image is first used in a frame (waits for the resource to be ready).
    /// Also requests that the underlying object lives at least as long as the given frame.
    /// This allows parts of the library to hold on to a raw handle to the resource and be sure that
    /// it won't be deleted until the given frame is complete. This reduces the number
    /// of Arcs<> in circulation.
    /// Advantage: no need to maintain a list of arcs borrowed for the frame.
    unsafe fn lock(
        &self,
        frame_number: FrameNumber,
    ) -> (Self::Proxy, Option<vk::Semaphore>, Option<vk::Semaphore>);
}
