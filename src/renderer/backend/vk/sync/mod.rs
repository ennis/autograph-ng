//! Frame-based synchronization primitives.
//!
//! Some objects are attached to a frame (or sequence of frames),
//! and should not be deleted until those frames are deleted.
use std::cell::Cell;
use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;
use crate::device::{
    Device, DeviceBoundObject, FrameNumber, FrameSynchronizedObject, VkDevice1,
    INVALID_FRAME_NUMBER,
};
use sid_vec::{Id, IdVec};

mod frame;

pub use self::frame::FrameFence;

//--------------------------------------------------------------------------------------------------
pub struct SignalSemaphore(vk::Semaphore);
pub struct WaitSemaphore(vk::Semaphore);

/// Safe cross-frame semaphores
pub struct FrameLock {
    semaphores: Vec<vk::Semaphore>,
    frame: Cell<FrameNumber>,
    current_index: Cell<usize>,
    initial: Cell<bool>,
}

pub struct FrameSyncSemaphores {
    pub wait_entry: Option<vk::Semaphore>,
    pub signal_exit: vk::Semaphore,
}

impl FrameLock {
    pub fn new(device: &Arc<Device>) -> FrameLock {
        let num_semaphores = (device.max_frames_in_flight() + 1) as usize;
        let mut semaphores = Vec::with_capacity(num_semaphores);
        let vkd = device.pointers();

        for i in 0..num_semaphores {
            let create_info = vk::SemaphoreCreateInfo {
                s_type: vk::StructureType::SemaphoreCreateInfo,
                p_next: ptr::null(),
                flags: vk::SemaphoreCreateFlags::empty(),
            };
            let semaphore = unsafe {
                vkd.create_semaphore(&create_info, None)
                    .expect("failed to create semaphore")
            };
            semaphores.push(semaphore);
        }

        FrameLock {
            semaphores,
            current_index: Cell::new(0),
            initial: Cell::new(true),
            frame: Cell::new(INVALID_FRAME_NUMBER),
        }
    }

    pub fn lock(&self, frame_number: FrameNumber) -> (Option<vk::Semaphore>, vk::Semaphore) {
        let entry_wait = if !self.initial.get() {
            self.semaphores[self.current_index.get()].into()
        } else {
            None
        };

        self.frame.set(frame_number);
        self.initial.set(false);
        let n = self.semaphores.len();
        self.current_index.set((self.current_index.get() + 1) % n);

        let exit_signal = self.semaphores[self.current_index.get()];
        (entry_wait, exit_signal)
    }

    pub fn locked_until(&self) -> FrameNumber {
        self.frame.get()
    }
}

/*struct Semaphore
{
    in_use: bool,
    signalled: bool,
    awaited: bool,
    last_used_frame: FrameNumber,
    last_used_queue: Option<u32>,
    semaphore: vk::Semaphore,
}

pub struct SemaphorePool
{
    pool: Vec<Semaphore>,
}

impl SemaphorePool
{
    fn new() -> SemaphorePool {
        SemaphorePool {
            semaphores: Vec::new(),
        }
    }

    fn request_semaphore(self: &Arc<Self>, signal_queue: u32) -> vk::Semaphore {
        if let Some(s) = self.pool.pop() {
            s.semaphore
        } else {

        }
    }
}*/

/*
impl Signal {
    /// Should not be called directly: use command submission wrappers.
    unsafe fn signal(self, current_frame: FrameNumber, queue_index: u32) -> vk::Semaphore {
        //self.pool.signal_semaphore(self.index, current_frame, queue_index)
        unimplemented!()
    }
}

impl Wait {
    /// Should not be called directly: use command submission wrappers.
    unsafe fn into_semaphore(self, current_frame: FrameNumber, queue_index: u32) -> vk::Semaphore {
        unimplemented!()
    }
}*/

//--------------------------------------------------------------------------------------------------
/*/// An object (or group of objects) that is bound to a particular frame:
/// i.e. that should not be deleted until the frame is retired.
pub(crate) struct FrameBoundObject<T> {
    frame_number: FrameNumber,
    obj: T,
}

impl<T> fmt::Debug for FrameBoundObject<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        unimplemented!()
    }
}*/

/*/// WaitLists are modified through FrameSync.
#[derive(Debug)]
pub(crate) struct SyncGroup<T> {
    objects: VecDeque<FrameBoundObject<T>>,
}

impl<T> SyncGroup<T> {
    pub(crate) fn new() -> SyncGroup<T> {
        SyncGroup {
            objects: VecDeque::new(),
        }
    }

    fn get_last_submitted(&self) -> Option<FrameNumber> {
        self.objects.back().map(|obj| obj.frame_number)
    }

    /// Synchronizes a wait list (dequeues objects bound to retired frames).
    pub(crate) fn sync_with(&mut self, frame_sync: &mut FrameSync, mut deleter: impl FnMut(T)) {
        loop {
            if let Some(front) = self.objects.pop_front() {
                if front.frame_number <= frame_sync.last_retired_frame {
                    deleter(front.obj)
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Enqueues an object into a wait list that is bound to the frame currently
    /// being submitted. Does not wait.
    pub(crate) fn enqueue(&mut self, obj: T, frame_sync: &mut FrameSync, deleter: impl FnMut(T)) {
        /*if let Some(frame) = wait_list.get_last_submitted() {
            assert!(frame < self.current_frame, "already submitted ");
        }*/
self.sync_with(frame_sync, deleter);
self.objects.push_back(FrameBoundObject {
frame_number: frame_sync.current_frame,
obj,
});
}
}
*/
