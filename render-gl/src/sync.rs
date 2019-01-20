use crate::api as gl;
use crate::api::types::*;
use crate::api::Gl;
use std::collections::vec_deque::VecDeque;
use std::time::Duration;

/// A synchronization primitive for objects used by the GPU.
///
/// This wrapper around OpenGL sync objects (fences) can be used to wait until the GPU has finished
/// using a particular object.
pub struct GpuSyncObject<T> {
    sync: GLsync,
    obj: T,
}

unsafe impl<T> Send for GpuSyncObject<T> {}

/// Error returned by GpuSyncObjects.
pub enum GpuSyncError {
    Timeout,
    Unspecified,
}

impl<T> GpuSyncObject<T> {
    /// Creates a new sync object that wraps the specified object.
    ///
    /// This function inserts a fence signal operation in the command stream.
    /// All operations using the object should be submitted before calling this function.
    pub fn new(gl: &Gl, obj: T) -> GpuSyncObject<T> {
        let sync = unsafe { gl.FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0) };
        GpuSyncObject { sync, obj }
    }

    /*pub fn wait_into_inner(self) -> Result<T, (GpuSyncError, Self)> {
        self.wait_into_inner_timeout(FENCE_CLIENT_WAIT_TIMEOUT)
    }*/

    /*pub fn try_wait_into_inner(self) -> Result<T, (GpuSyncError, Self)> {
        self.wait_into_inner_timeout(0)
    }*/

    /*pub fn wait(&self) -> Result<(), GpuSyncError> {
        self.wait_timeout(FENCE_CLIENT_WAIT_TIMEOUT)
    }*/

    /// Waits for the submitted fence to be signalled (i.e. the GPU and driver has finished using
    /// the object).
    pub fn try_wait(&self, gl: &Gl) -> Result<(), GpuSyncError> {
        self.wait_timeout(gl, 0)
    }

    /// Extracts the wrapper object without waiting for the driver.
    pub unsafe fn into_inner_unsynchronized(self, gl: &Gl) -> T {
        gl.DeleteSync(self.sync);
        self.obj
    }

    //---------------------------------------
    /*fn wait_into_inner_timeout(self, timeout: u64) -> Result<T, (GpuSyncError, Self)> {
        match self.wait_timeout(timeout) {
            Ok(()) => {
                unsafe {
                    gl::DeleteSync(self.sync);
                }
                Ok(self.obj)
            }
            Err(e) => Err((e, self)),
        }
    }*/

    fn wait_timeout(&self, gl: &Gl, timeout: u64) -> Result<(), GpuSyncError> {
        let wait_result =
            unsafe { gl.ClientWaitSync(self.sync, gl::SYNC_FLUSH_COMMANDS_BIT, timeout) };

        if wait_result == gl::CONDITION_SATISFIED || wait_result == gl::ALREADY_SIGNALED {
            Ok(())
        } else if wait_result == gl::WAIT_FAILED {
            Err(GpuSyncError::Unspecified)
        } else {
            // Timeout
            Err(GpuSyncError::Timeout)
        }
    }
}

struct SyncPoint {
    sync: GLsync,
    value: u64,
}

unsafe impl Send for SyncPoint {}

/// Synchronization timelines.
///
/// A timeline is synchronization primitive that contains a monotonically increasing 64-bit integer.
/// 'Signal' operations inside the GPU command stream increase this value.
/// The application can then wait for a specific timeline value to ensure that all commands prior
/// to the corresponding signal operation have finished.
///
/// This is basically an emulation of D3D12 Fences, and of Vulkan's proposed 'timeline semaphores'.
pub struct Timeline {
    sync_points: VecDeque<SyncPoint>,
    current_value: u64,
}

impl Timeline {
    /// Creates a new timeline with the specified initial value.
    pub fn new(init_value: u64) -> Timeline {
        Timeline {
            sync_points: VecDeque::new(),
            current_value: init_value,
        }
    }

    /// Signals the timeline.
    ///
    /// The timeline value is increased once all GPU operations submitted prior to this call
    /// have completed.
    pub fn signal(&mut self, gl: &Gl, value: u64) {
        let sync = unsafe { gl.FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0) };
        self.sync_points.push_back(SyncPoint { sync, value });
    }

    /// Waits for the given value to be reached (on the application side).
    ///
    /// Timeout is for a single ClientWaitSync only: there may be more than one.
    /// Returns true if value reached, false if timeout. Panics if wait failed.
    pub fn client_sync(&mut self, gl: &Gl, value: u64, timeout: Duration) -> bool {
        while self.current_value < value {
            //debug!("client_sync current {} target {}", self.current_value, value);
            if let Some(target) = self.sync_points.front() {
                let timeout_ns = timeout.as_nanos();
                assert!(timeout_ns <= u64::max_value().into());
                let timeout_ns = timeout_ns as u64;
                let wait_result = unsafe {
                    gl.ClientWaitSync(target.sync, gl::SYNC_FLUSH_COMMANDS_BIT, timeout_ns)
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
                gl.DeleteSync(sp.sync);
            }
        }
        true
    }
}
