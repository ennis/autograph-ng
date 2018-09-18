//! Frame-based synchronization primitives.
//!
//! Some objects are attached to a frame (or sequence of frames),
//! and should not be deleted until those frames are deleted.

use std::collections::VecDeque;

use ash::vk;
use ash::version::DeviceV1_0;

use context::{VkDevice1, FrameNumber, FRAME_NONE};

/// An object (or group of objects) that is bound to a particular frame:
/// i.e. that should not be deleted until the frame is retired.
pub(crate) struct FrameBoundObject<T>
{
    frame_number: FrameNumber,
    obj: T
}

/// An object used to wait for a frame to complete.
pub(crate) struct FrameSync
{
    /// The number of the frame **being submitted**.
    current_frame: FrameNumber,
    /// The last frame that has been **fully completed** (retired).
    /// Must have `current_frame > last_retired_frame`.
    last_retired_frame: FrameNumber,
    /// Fences for all frames in flight (submitted, but not completed).
    /// Range ]last_retired_frame, current_frame]
    fences: VecDeque<Vec<vk::Fence>>,
}

/// WaitLists are modified through FrameSync.
struct WaitList<T>
{
    objects: VecDeque<FrameBoundObject<T>>,
}

const FRAME_FENCE_WAIT_TIMEOUT: u64 = 1_000_000_000;  // 1sec timeout

impl FrameSync
{
    /// Creates a new FrameSync, setting the current frame number.
    pub(crate) fn new(current_frame: FrameNumber, max_frames_in_flight: u32) -> FrameSync
    {
        let mut fences =  VecDeque::with_capacity((max_frames_in_flight + 1) as usize);
        fences.push_front(Vec::new());
        FrameSync {
            current_frame,
            last_retired_frame: FRAME_NONE,
            fences
        }
    }

    /// Adds a fence that should we waited upon for completion of the frame being submitted.
    /// Acquires ownership of the fence.
    pub(crate) fn add_frame_fence(&mut self, fence: vk::Fence)
    {
        let mut front = self.fences.front_mut().expect("empty queue");
        front.push(fence);
    }


    /// Synchronizes a wait list (dequeues objects bound to retired frames).
    pub(crate) fn sync_wait_list<T>(&self, wait_list: &mut WaitList<T>, deleter: impl FnOnce(T)) {
        loop {
            if let Some(front) = wait_list.objects.pop_front() {
                if front.frame_number <= self.last_retired_frame {
                    deleter(front)
                } else {
                    break
                }
            } else {
                break
            }
        }
    }

    /// Enqueues an object into a wait list that is bound to the frame currently
    /// being submitted. Does not wait.
    pub(crate) fn enqueue<T>(&self, wait_list: &mut WaitList<T>, obj: T, deleter: impl FnOnce(T))
    {
        self.sync_wait_list(wait_list, deleter);
        wait_list.objects.push_back(obj);
    }

    /// Signals that the submission of the current frame is complete, and increases the
    /// current frame index.
    pub(crate) fn complete_frame(&mut self)
    {
        self.fences.push_front(Vec::new());
        self.current_frame = self.current_frame.next();
    }

    /// Checks if the given frame has completed.
    /// If `device` is not `None` then calls `vkGetFenceStatus` if necessary.
    /// Otherwise, just checks that `frame <= self.last_retired_frame`.
    pub(crate) fn check_frame_complete(&mut self, frame: FrameNumber, vkd: Option<VkDevice1>) -> bool {
        if frame <= self.last_retired_frame {
            return true
        }

        if let Some(vkd) = vkd {
            let i = ((frame.0 - self.last_retired_frame.0) - 1) as usize;
            let wait_fences = self.fences[i].as_slice();
            unsafe {
                vkd.wait_for_fences(wait_fences, true, 0).is_ok() // FIXME handle error returns
            }
        } else {
            false
        }
    }

    /// Updates `last_retired_frame` by waiting on the fences associated to the given frame number.
    pub(crate) fn wait_on_frame_complete(&mut self, frame: FrameNumber, vkd: VkDevice1) {
        assert!(frame < self.current_frame, "cannot wait on not yet submitted frames");

        if frame <= self.last_retired_frame {
            // frame already retired, no need to wait
            return;
        }

        // index of the fences that need waiting.
        let i = ((frame.0 - self.last_retired_frame.0) - 1) as usize;
        {
            let wait_fences = self.fences[i].as_slice();
            unsafe {
                let result = vkd.wait_for_fences(wait_fences, true, FRAME_FENCE_WAIT_TIMEOUT);    // FIXME handle error returns
                match result {
                    Err(vk::Result::Timeout) => { panic!("timeout waiting for frame fence"); }
                    Err(_) => { panic!("error waiting for frame fence"); }
                    Ok(_) => {}
                }
            }
            // drop borrow of fences
        }

        for _ in 0..=i {
            let fences = self.fences.pop_back().unwrap();
            for &f in fences.iter() {
                // safe to call because we've waited for the frame to complete.
                unsafe {
                    vkd.destroy_fence(f, None);
                }
            }
        }

        self.last_retired_frame = frame;
    }

    /// Returns the frame number of the frame being submitted.
    pub(crate) fn current_frame(&self) -> FrameNumber {
        self.current_frame
    }


    /// Returns the last retired frame.
    pub(crate) fn last_retired_frame(&self) -> FrameNumber {
        self.last_retired_frame
    }

}
/*
impl<T> FrameBoundObject<T>
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
}*/



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