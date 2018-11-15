use std::collections::VecDeque;
use std::fmt;

use ash::version::DeviceV1_0;
use ash::vk;

use crate::device::{FrameNumber, VkDevice1, INVALID_FRAME_NUMBER};

/// An object used to wait for a frame to complete.
pub struct FrameFence {
    /// The number of the frame **being submitted**.
    current_frame: FrameNumber,
    /// The last frame that has been **fully completed** (retired).
    /// Must have `current_frame > last_retired_frame`.
    last_retired_frame: FrameNumber,
    /// Fences for all frames in flight (submitted, but not completed).
    /// Range ]last_retired_frame, current_frame]
    fences: VecDeque<Vec<vk::Fence>>,
}

const FRAME_FENCE_WAIT_TIMEOUT: u64 = 1_000_000_000; // 1sec timeout

impl FrameFence {
    /// Creates a new FrameSync, setting the current frame number.
    pub fn new(current_frame: FrameNumber, max_frames_in_flight: u32) -> FrameFence {
        let mut fences = VecDeque::with_capacity((max_frames_in_flight + 1) as usize);
        fences.push_back(Vec::new());
        FrameFence {
            current_frame,
            last_retired_frame: INVALID_FRAME_NUMBER,
            fences,
        }
    }

    /// Adds a fence that should we waited upon for completion of the frame being submitted.
    /// Acquires ownership of the fence.
    pub fn add_frame_fence(&mut self, fence: vk::Fence) {
        let mut back = self.fences.back_mut().expect("empty queue");
        back.push(fence);
    }

    /// Signals that the submission of the current frame is complete, and increases the
    /// current frame index.
    pub fn complete_frame(&mut self) {
        self.fences.push_front(Vec::new());
        self.current_frame.0 = self.current_frame.0 + 1;
    }

    /// Checks if the given frame has completed.
    /// If `device` is not `None` then calls `vkGetFenceStatus` if necessary.
    /// Otherwise, just checks that `frame <= self.last_retired_frame`.
    pub fn check_frame_complete(&mut self, frame: FrameNumber, vkd: Option<VkDevice1>) -> bool {
        if frame <= self.last_retired_frame {
            return true;
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
    pub fn wait_on_frame_complete(&mut self, frame: FrameNumber, vkd: VkDevice1) {
        assert!(
            frame < self.current_frame,
            "cannot wait on not yet submitted frames"
        );

        if frame <= self.last_retired_frame {
            // frame already retired, no need to wait
            return;
        }

        // index of the fences that need waiting.
        let i = ((frame.0 - self.last_retired_frame.0) - 1) as usize;
        {
            let wait_fences = self.fences[i].as_slice();
            unsafe {
                let result = vkd.wait_for_fences(wait_fences, true, FRAME_FENCE_WAIT_TIMEOUT); // FIXME handle error returns
                match result {
                    Err(vk::Result::Timeout) => {
                        panic!("timeout waiting for frame fence");
                    }
                    Err(_) => {
                        panic!("error waiting for frame fence");
                    }
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
    pub fn current_frame(&self) -> FrameNumber {
        self.current_frame
    }

    /// Returns the last retired frame.
    pub fn last_retired_frame(&self) -> FrameNumber {
        self.last_retired_frame
    }
}
