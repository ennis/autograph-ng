//! Vulkan memory allocators

use std::error;
use std::fmt;
use std::ops::Range;
use std::ptr;

use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;
use ash::vk;
use sid_vec::{Id, IdVec};

use context::*;
use handle::OwningHandle;

#[derive(Debug, Clone)]
pub enum AllocError {
    NoSuitableMemoryType,
    //OutOfMemory,
    Other(vk::Result),
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "allocation failed")
    }
}

impl error::Error for AllocError {
    fn description(&self) -> &str {
        "allocation failed"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

pub(crate) struct Allocation {
    /// Associated device memory object.
    device_memory: vk::DeviceMemory,
    /// Offset within the device memory.
    range: Range<u64>,
}

fn align_offset(size: u64, align: u64, space: Range<u64>) -> Option<u64> {
    assert!(align.is_power_of_two(), "alignment must be a power of two");
    let mut off = space.start & (align - 1);
    if off > 0 {
        off = align - off;
    }
    if space.start + off + size > space.end {
        None
    } else {
        Some(space.start + off)
    }
}

/// A block of device memory in a pool.
struct Block {
    device_memory: OwningHandle<vk::DeviceMemory>,
}

/// Linear memory pools.
/// Only appends to the end, allocate blocks when necessary.
/// Free is a no-op.
struct LinearMemoryPool {
    memory_type_index: u32,
    block_size: u64,
    front_block: u32,
    front_ptr: u64,
    blocks: Vec<Block>,
}

impl LinearMemoryPool {
    fn new(memory_type_index: u32, block_size: u64) -> LinearMemoryPool {
        LinearMemoryPool {
            memory_type_index,
            block_size,
            blocks: Vec::new(),
            front_block: 0,
            front_ptr: 0,
        }
    }

    /// Should be mostly safe.
    fn new_block(&mut self, vkd: &VkDevice1) {
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

        self.blocks.push(Block { device_memory });

        self.front_ptr = 0;
    }

    /// Should be mostly safe.
    fn allocate(&mut self, size: u64, align: u64, vkd: &VkDevice1) -> Option<Allocation> {
        assert!(align.is_power_of_two(), "alignment must be a power of two");

        if size > self.block_size {
            None
        }

        if self.blocks.is_empty() {
            self.new_block(vkd);
        }

        if let Some(ptr) = align_offset(size, align, self.front_ptr..self.block_size) {
            // suballocate
            Some(Allocation {
                device_memory: self.blocks.last().unwrap().device_memory.get(),
                range: ptr..(ptr + size),
            })
        } else {
            self.new_block(vkd);
            let ptr = self.front_ptr;
            self.front_ptr += size;
            Some(Allocation {
                device_memory: self.blocks.last().unwrap().device_memory.get(),
                range: ptr..(ptr + size),
            })
        }
    }

    /// Unsafe because reasons.
    unsafe fn deallocate_all(&mut self, vkd: &VkDevice1) {
        for b in self.blocks.drain(..) {
            b.device_memory.destroy(|device_memory| {
                vkd.free_memory(device_memory, None);
            });
        }
    }
}

pub fn find_compatible_memory_type_index(
    memory_types: &[vk::MemoryType],
    required_flags: vk::MemoryPropertyFlags,
    preferred_flags: vk::MemoryPropertyFlags,
    memory_type_bits: u32,
) -> Option<u32> {
    memory_types
        .iter()
        .enumerate()
        .filter(|(_, mt)| mt.property_flags.subset(required_flags | preferred_flags))
        .chain(
            memory_types
                .iter()
                .enumerate()
                .filter(|(_, mt)| mt.property_flags.subset(required_flags)),
        ).filter(|&(mt_index, _)| (1 << (mt_index as u32)) & memory_type_bits != 0)
        .next()
        .map(|(mt_index, _)| mt_index as u32)
}

pub fn is_compatible_memory_type(
    memory_types: &[vk::MemoryType],
    memory_type_index: u32,
    memory_type_bits: u32,
    flags: vk::MemoryPropertyFlags,
) -> bool {
    ((memory_type_bits & (1 << (mt_index as u32))) != 0) && memory_types[memory_type_index as usize]
        .property_flags
        .subset(flags)
}

/// Memory allocator for vulkan device heaps.
pub struct Allocator {
    memory_types: Vec<vk::MemoryType>,
    memory_heaps: Vec<vk::MemoryHeap>,
    /// Default pools for all memory types.
    default_pools: Vec<LinearMemoryPool>,
}

/// An allocated block of memory that is bound to a frame.
/// I.e.
/// TODO generalize this concept to other resources in a frame.
pub struct FrameBoundAllocation {
    /// A block of device memory.
    memory: vk::DeviceMemory,
    /// The last frame that used this alloc.
    last_used_by: FrameNumber,
}

/// A transient memory pool, that contains allocations for a frame.
pub struct TransientMemoryPool {
    /// Memory blocks in use.
    /// DeviceMemory + last frame that used this memory block
    /// If the frame is completed, then can delete.
    allocated: Vec<FrameBoundAllocation>,
    /// Memory blocks available.
    free: Vec<FrameBoundAllocation>,
}

pub struct AllocationCreateInfo {
    size: usize,
    required_flags: vk::MemoryPropertyFlags,
    preferred_flags: vk::MemoryPropertyFlags,
    memory_type_bits: u32,
}

impl Allocator {
    pub fn new(
        vki: &VkInstance1,
        vkd: &VkDevice1,
        physical_device: vk::PhysicalDevice,
    ) -> Allocator {
        // query all memory types
        let p = vki.get_physical_device_memory_properties(physical_device);
        let memory_types = p.memory_types[0..p.memory_type_count as usize].to_vec();
        let memory_heaps = p.memory_heaps[0..p.memory_heap_count as usize].to_vec();

        Allocator {
            memory_types,
            memory_heaps,
            to_deallocate: Vec::new(),
        }
    }

    pub fn create_transient_memory_pool(&self, vkd: &VkDevice1) -> TransientMemoryPool {
        TransientMemoryPool {
            allocated: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn allocate_transient_image(
        &self,
        pool: &mut TransientMemoryPool,
        info: &AllocationCreateInfo,
        current_frame: FrameNumber,
        vkd: &VkDevice1,
        img: vk::Image,
    ) {
        // allocation algorithm
        // - find a free memory block in the
    }

    pub fn free_transient(&self, pool: &mut TransientMemoryPool) {}

    /// Note: deallocation is deferred.
    pub fn free_transient_memory_pool(
        &self,
        pool: &mut TransientMemoryPool,
        vkd: &VkDevice1,
        current_frame: FrameNumber,
    ) {

    }

    pub fn allocate(
        &self,
        vkd: &VkDevice1,
        info: &AllocationCreateInfo,
    ) -> Result<vk::DeviceMemory, AllocError> {
        let mut found_suitable_memory_type = false;
        // find a suitable memory type
        // first, look for mem types with required + preferred, then look again with only required
        // keep only memtypes that are compatible with the type bits.
        for (mt_index, mt) in self
            .memory_types
            .iter()
            .enumerate()
            .filter(|(_, mt)| {
                mt.property_flags
                    .subset(info.required_flags | info.preferred_flags)
            }).chain(
                self.memory_types
                    .iter()
                    .enumerate()
                    .filter(|(_, mt)| mt.property_flags.subset(info.required_flags)),
            ).filter(|&(mt_index, _)| (1 << (mt_index as u32)) & info.memory_type_bits != 0)
        {
            found_suitable_memory_type = true;
            debug!(
                "alloc: allocating {} bytes in memory type {}",
                info.size, mt_index
            );
            //
            let vk_alloc_info = vk::MemoryAllocateInfo {
                allocation_size: info.size as vk::types::DeviceSize,
                memory_type_index: mt_index as u32,
                p_next: ptr::null(),
                s_type: vk::StructureType::MemoryAllocateInfo,
            };

            let mem = unsafe {
                vkd.allocate_memory(&vk_alloc_info, None)
                    .map_err(|e| AllocError::Other(e))?
            };
            return Ok(mem);
        }

        if found_suitable_memory_type {
            Err(AllocError::Other(vk::Result::Success))
        } else {
            Err(AllocError::NoSuitableMemoryType)
        }
    }
}
