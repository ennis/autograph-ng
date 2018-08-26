//use super::context::Context;
/*use gl;
use gl::types::*;
use std::collections::vec_deque::VecDeque;

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) struct FenceValue(pub(crate) i64);

struct SyncPoint {
    sync: GLsync,
    value: FenceValue,
}

pub(crate) struct Fence {
    sync_points: VecDeque<SyncPoint>,
    current_value: FenceValue,
    next_value: FenceValue,
}

impl Fence {
    pub(crate) fn new(init_value: FenceValue) -> Fence {
        Fence {
            sync_points: VecDeque::new(),
            current_value: init_value,
            next_value: FenceValue(init_value.0 + 1),
        }
    }

    pub(crate) fn advance_async(&mut self) -> FenceValue {
        let sync = unsafe { gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0) };
        self.sync_points.push_back(SyncPoint {
            sync,
            value: self.next_value,
        });
        self.next_value.0 += 1;
        self.next_value
    }

    pub(crate) fn next_value(&self) -> FenceValue {
        self.next_value
    }

    // This should not be mut!
    pub(crate) fn current_value(&mut self) -> FenceValue {
        while !self.sync_points.is_empty() {
            if !self.wait_one(0) {
                break;
            }
        }
        self.current_value
    }

    // Returns true if the current_value was bumped
    fn wait_one(&mut self, timeout: u64) -> bool {
        let advanced = if let Some(target_sync) = self.sync_points.front() {
            let wait_result = unsafe {
                gl::ClientWaitSync(target_sync.sync, gl::SYNC_FLUSH_COMMANDS_BIT, timeout)
            };

            if wait_result == gl::CONDITION_SATISFIED || wait_result == gl::ALREADY_SIGNALED {
                self.current_value = target_sync.value;
                true
            } else if wait_result == gl::WAIT_FAILED {
                panic!("Fence wait failed")
            } else {
                false
            }
        } else {
            // nothing in the wait list
            false
        };
        if advanced {
            let sp = self.sync_points.pop_front().unwrap();
            unsafe {
                gl::DeleteSync(sp.sync);
            }
        }
        advanced
    }

    fn wait_until(&mut self, _timeout: u64) {
        unimplemented!()
    }
}*/

//pub fn signal_fence
