use std::ptr;
use std::sync::Arc;

use super::{align_offset, MemoryBlock};
use crate::renderer::vk::handle::VkHandle;

use ash;
use ash::vk;

/// A block of device memory in a pool.
struct Block {
    device_memory: VkHandle<vk::DeviceMemory>,
}

/// Linear memory pools.
/// Only appends to the end, allocate blocks when necessary.
/// Free is a no-op.
pub struct LinearMemoryPool {
    memory_type_index: u32,
    block_size: u64,
    front_block: u32,
    front_ptr: u64,
    blocks: Vec<Block>,
}

impl LinearMemoryPool {
    pub fn new(memory_type_index: u32, block_size: u64) -> LinearMemoryPool {
        LinearMemoryPool {
            memory_type_index,
            block_size,
            blocks: Vec::new(),
            front_block: 0,
            front_ptr: 0,
        }
    }

    /// Should be mostly safe.
    fn new_block(&mut self, vkd: &ash::Device) {
        let alloc_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MemoryAllocateInfo,
            p_next: ptr::null(),
            allocation_size: self.block_size,
            memory_type_index: self.memory_type_index,
        };

        let device_memory = unsafe {
            vkd.allocate_memory(&alloc_info, None)
                .expect("allocation failed")
        };

        self.blocks.push(Block {
            device_memory: VkHandle::new(device_memory),
        });

        self.front_ptr = 0;
    }

    /// Should be mostly safe.
    pub fn allocate(&mut self, vkd: &ash::Device, size: u64, alignment: u64) -> Option<MemoryBlock> {
        assert!(
            alignment.is_power_of_two(),
            "alignment must be a power of two"
        );

        if size > self.block_size {
            return None;
        }

        if self.blocks.is_empty() {
            self.new_block(vkd);
        }

        if let Some(ptr) = align_offset(size, alignment, self.front_ptr..self.block_size) {
            // suballocate
            Some(MemoryBlock {
                device_memory: self.blocks.last().unwrap().device_memory.get(),
                range: ptr..(ptr + size),
            })
        } else {
            self.new_block(vkd);
            let ptr = self.front_ptr;
            self.front_ptr += size;
            Some(MemoryBlock {
                device_memory: self.blocks.last().unwrap().device_memory.get(),
                range: ptr..(ptr + size),
            })
        }
    }

    /*/// Unsafe because reasons.
    pub unsafe fn deallocate_all(&mut self) {
        for b in self.blocks.drain(..) {
            b.device_memory.destroy(|device_memory| {
                self.device.pointers().free_memory(device_memory, None);
            });
        }
    }*/
}
