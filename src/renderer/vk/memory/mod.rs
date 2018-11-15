//! Vulkan memory allocators
use std::cell::RefCell;
use std::error;
use std::fmt;
use std::ops::Range;
use std::ptr;
use std::sync::Arc;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use sid_vec::{Id, IdVec};

use crate::device::Device;

mod linear_pool;
use self::linear_pool::LinearMemoryPool;


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

pub struct MemoryBlock {
    /// Associated device memory object.
    pub device_memory: vk::DeviceMemory,
    /// Offset within the device memory.
    pub range: Range<u64>,
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

/*pub fn find_compatible_memory_type_index(
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
}*/

pub fn compatible_memory_types<'a>(
    memory_types: &'a [vk::MemoryType],
    required_flags: vk::MemoryPropertyFlags,
    preferred_flags: vk::MemoryPropertyFlags,
    memory_type_bits: u32,
) -> impl Iterator<Item = (u32, &'a vk::MemoryType)> + 'a {
    memory_types
        .iter()
        .enumerate()
        .filter(move |(_, mt)| mt.property_flags.subset(required_flags | preferred_flags))
        .chain(
            memory_types
                .iter()
                .enumerate()
                .filter(move |(_, mt)| mt.property_flags.subset(required_flags)),
        )
        .filter(move |&(mt_index, _)| (1 << (mt_index as u32)) & memory_type_bits != 0)
        .map(move |(mt_index, mt)| (mt_index as u32, mt))
}

pub fn is_compatible_memory_type(
    memory_types: &[vk::MemoryType],
    memory_type_index: u32,
    memory_type_bits: u32,
    flags: vk::MemoryPropertyFlags,
) -> bool {
    ((memory_type_bits & (1 << (memory_type_index as u32))) != 0)
        && memory_types[memory_type_index as usize]
            .property_flags
            .subset(flags)
}

pub struct AllocationCreateInfo {
    size: u64,
    alignment: u64,
    required_flags: vk::MemoryPropertyFlags,
    preferred_flags: vk::MemoryPropertyFlags,
    memory_type_bits: u32,
}

/// Memory allocator for vulkan device heaps.
pub struct MemoryPool {
    device: Arc<Device>,
    memory_types: Vec<vk::MemoryType>,
    memory_heaps: Vec<vk::MemoryHeap>,
    large_alloc_size: u64,
    default_pools: RefCell<Vec<LinearMemoryPool>>,
}

impl MemoryPool {
    pub fn new(device: &Arc<Device>, block_size: u64) -> MemoryPool {
        // query all memory types
        let vki = device.instance().pointers();

        let p = vki.get_physical_device_memory_properties(device.physical_device());
        let memory_types = p.memory_types[0..p.memory_type_count as usize].to_vec();
        let memory_heaps = p.memory_heaps[0..p.memory_heap_count as usize].to_vec();

        MemoryPool {
            device: device.clone(),
            memory_types,
            memory_heaps,
            large_alloc_size: block_size,
            default_pools: RefCell::new(
                (0..p.memory_heap_count)
                    .map(|mt_index| LinearMemoryPool::new(&device, mt_index, block_size))
                    .collect(),
            ),
        }
    }

    pub fn allocate_memory(&self, info: &AllocationCreateInfo) -> Result<MemoryBlock, AllocError> {
        if info.size >= self.large_alloc_size {
            return self.allocate_dedicated(info);
        }

        for (mt_index, mt) in compatible_memory_types(
            &self.memory_types,
            info.required_flags,
            info.preferred_flags,
            info.memory_type_bits,
        ) {
            if let Some(alloc) = self.default_pools.borrow_mut()[mt_index as usize]
                .allocate(info.size, info.alignment)
            {
                return Ok(alloc);
            }
        }

        // resort to dedicated allocation
        self.allocate_dedicated(info)
    }

    /// Frees the specified block of memory.
    pub fn free_memory(&mut self, memory: MemoryBlock) {
        // No-op for now
    }

    pub fn allocate_dedicated(
        &self,
        info: &AllocationCreateInfo,
    ) -> Result<MemoryBlock, AllocError> {
        let mut found_suitable_memory_type = false;
        // find a suitable memory type
        // first, look for mem types with required + preferred, then look again with only required
        // keep only memtypes that are compatible with the type bits.
        for (mt_index, mt) in compatible_memory_types(
            &self.memory_types,
            info.required_flags,
            info.preferred_flags,
            info.memory_type_bits,
        ) {
            found_suitable_memory_type = true;
            debug!(
                "alloc: allocating {} bytes in memory type {}",
                info.size, mt_index
            );
            //
            let vk_alloc_info = vk::MemoryAllocateInfo {
                allocation_size: info.size,
                memory_type_index: mt_index as u32,
                p_next: ptr::null(),
                s_type: vk::StructureType::MemoryAllocateInfo,
            };

            let device_memory = unsafe {
                self.device
                    .pointers()
                    .allocate_memory(&vk_alloc_info, None)
                    .map_err(|e| AllocError::Other(e))?
            };
            return Ok(MemoryBlock {
                device_memory,
                range: 0..info.size,
            });
        }

        if found_suitable_memory_type {
            Err(AllocError::Other(vk::Result::Success))
        } else {
            Err(AllocError::NoSuitableMemoryType)
        }
    }
}