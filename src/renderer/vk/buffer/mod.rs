//! Buffers
//!
use std::ptr;
use std::sync::Arc;

use ash::vk;

use crate::device::Device;
use crate::handle::VkHandle;
use crate::memory::{AllocationCreateInfo, MemoryBlock, MemoryPool};
use crate::resource::Resource;
//use crate::sync::SyncGroup;

mod immutable;
mod unbound;

pub trait BufferDescription {
    fn size(&self) -> u64;
    fn usage(&self) -> vk::BufferUsageFlags;
}

pub trait Buffer: BufferDescription {
    fn device(&self) -> &Device;
}
