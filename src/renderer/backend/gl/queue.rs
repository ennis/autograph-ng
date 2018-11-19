use super::context::{Context, ContextConfig};
use super::fence::{Fence, FenceValue};
use super::upload_buffer::UploadBuffer;
use std::cell::RefCell;
use std::collections::VecDeque;

use super::buffer::*;
use super::texture::*;

pub(super) struct FrameResources {
    pub(super) ref_buffers: Vec<BufferAny>,
    pub(super) ref_textures: Vec<TextureAny>,
}

struct SubmittedFrame {
    sync: FenceValue,
    resources: FrameResources,
}

/// Represents a succession of frames
pub struct Queue {
    pub(super) gctx: Context,
    pub(super) fence: RefCell<Fence>,
    // submitted but not completed frames, hold refs to resources
    submitted_frames: RefCell<VecDeque<SubmittedFrame>>,
    default_upload_buffer: UploadBuffer,
}

pub const DEFAULT_UPLOAD_BUFFER_SIZE: usize = 3 * 1024 * 1024;

impl Queue {
    /// Creates a new queue
    pub fn new(ctx: &Context) -> Queue {
        Queue {
            gctx: ctx.clone(),
            fence: RefCell::new(Fence::new(&ctx.clone(), FenceValue(-1))),
            submitted_frames: RefCell::new(VecDeque::new()),
            default_upload_buffer: UploadBuffer::new(ctx, DEFAULT_UPLOAD_BUFFER_SIZE),
        }
    }

    /// Returns the context the queue was created with
    pub fn context(&self) -> &Context {
        &self.gctx
    }

    pub fn current_frame_index(&self) -> u64 {
        self.fence.borrow().next_value().0 as u64
    }

    /// Returns -1 if no frame is done yet
    /// (more practical than returning an Option<FenceValue>)
    pub fn last_completed_frame(&self) -> FenceValue {
        self.fence.borrow_mut().current_value()
    }

    /// TODO document
    pub fn next_frame_fence_value(&self) -> FenceValue {
        self.fence.borrow().next_value()
    }

    /// Submit the set of resources that are referenced by the current frame,
    /// and advances the fence value
    /// Called internally by `Frame::submit()`
    pub(super) fn submit(&mut self, resources: FrameResources) {
        let mut fence = self.fence.borrow_mut();
        let current_sync = fence.current_value();
        // collect frames that are done
        let mut submitted_frames = self.submitted_frames.borrow_mut();
        submitted_frames.retain(|frame| frame.sync > current_sync);
        // add the new one
        submitted_frames.push_back(SubmittedFrame {
            sync: fence.next_value(),
            resources,
        });
        //debug!("Number of live submitted frames: {}", submitted_frames.len());
        // put a sync point in the command stream and bump the frame index
        fence.advance_async();
    }

    pub(super) fn default_upload_buffer(&self) -> &UploadBuffer {
        &self.default_upload_buffer
    }
}

pub fn create_context_and_queue(_config: &ContextConfig) -> (Context, Queue) {
    unimplemented!()
}
