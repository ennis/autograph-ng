use std::collections::vec_deque::VecDeque;

use crate::renderer::backend::gl::api as gl;
use crate::renderer::backend::gl::api::types::*;

struct SyncPoint {
    sync: GLsync,
    value: u64,
}

unsafe impl Send for SyncPoint {}

pub struct Timeline {
    sync_points: VecDeque<SyncPoint>,
    current_value: u64,
}

const FENCE_CLIENT_WAIT_TIMEOUT: u64 = 1_000_000_000;

#[derive(Copy, Clone, Debug)]
pub enum Timeout {
    Infinite,
    Nanoseconds(u64),
}

impl Timeline {
    pub fn new(init_value: u64) -> Timeline {
        Timeline {
            sync_points: VecDeque::new(),
            current_value: init_value,
        }
    }

    pub fn signal(&mut self, value: u64) {
        let sync = unsafe { gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0) };
        self.sync_points.push_back(SyncPoint { sync, value });
    }

    /// Waits for the given value. (implies driver sync)
    /// Timeout is for a single ClientWaitSync only: there may be more than one.
    /// Returns true if value reached, false if timeout. Panics if wait failed.
    pub fn client_sync(&mut self, value: u64, timeout: Timeout) -> bool {
        while self.current_value < value {
            //debug!("client_sync current {} target {}", self.current_value, value);
            if let Some(target) = self.sync_points.front() {
                let timeout = match timeout {
                    Timeout::Infinite => FENCE_CLIENT_WAIT_TIMEOUT,
                    Timeout::Nanoseconds(timeout) => timeout,
                };
                let wait_result = unsafe {
                    gl::ClientWaitSync(target.sync, gl::SYNC_FLUSH_COMMANDS_BIT, timeout)
                };
                if wait_result == gl::CONDITION_SATISFIED || wait_result == gl::ALREADY_SIGNALED {
                    self.current_value = target.value;
                } else if wait_result == gl::WAIT_FAILED {
                    panic!("fence wait failed")
                } else {
                    // Timeout
                    return false;
                }
            } else {
                // nothing in the wait list, and value not reached
                panic!("deadlocked timeline")
            }

            let sp = self.sync_points.pop_front().unwrap();
            unsafe {
                gl::DeleteSync(sp.sync);
            }
        }
        true
    }

    pub fn driver_sync(&mut self, value: u64, timeout: Timeout) -> bool {
        unimplemented!()
    }
}
