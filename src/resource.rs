//! Common resource trait.

use std::cell::Cell;
use std::mem;
use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;
use downcast_rs::Downcast;
use slotmap::Key;

use alloc::Allocation;
use context::{FrameNumber, VkDevice1, FRAME_NONE};
use handle::OwningHandle;
use sync::{FrameSync, SyncGroup};

//--------------------------------------------------------------------------------------------------
// Resources

/// Trait representing the shared functionality and properties of resources (buffers and images).
pub trait Resource {
    type CreateInfo: Clone;

    /// Gets the name of the resource.
    /// Note that the name does not uniquely identifies a resource,
    /// as it does not need to be unique among all resources.
    fn name(&self) -> &str;

    /// The frame in which the resource was last used.
    fn last_used_frame(&self) -> FrameNumber;

    fn create_info(&self) -> &Self::CreateInfo;
}

