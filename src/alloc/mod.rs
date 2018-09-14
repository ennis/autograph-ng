//! Vulkan memory allocators

use std::fmt;
use std::error;
use std::ptr;

use ash::vk;
use ash::version::InstanceV1_0;
use ash::version::DeviceV1_0;
use context::*;

#[derive(Debug, Clone)]
// Define our error types. These may be customized for our error handling cases.
// Now we will be able to write our own errors, defer to an underlying error
// implementation, or do something in between.
pub enum AllocError
{
    NoSuitableMemoryType,
    //OutOfMemory,
    Other(vk::Result)
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
        // Generic error, underlying cause isn't tracked.
        None
    }
}

/// Memory allocator for vulkan device heaps.
/// Bound to the lifetime of the device.
pub struct Allocator
{
    memory_types: Vec<vk::MemoryType>,
    memory_heaps: Vec<vk::MemoryHeap>,
}

pub struct AllocationCreateInfo
{
    size: usize,
    required_flags: vk::MemoryPropertyFlags,
    preferred_flags: vk::MemoryPropertyFlags,
    memory_type_bits: u32,
}

impl Allocator
{
    pub fn new(vki: &VkInstance1, vkd: &VkDevice1, physical_device: vk::PhysicalDevice) -> Allocator {
        // query all memory types
        let p = vki.get_physical_device_memory_properties(physical_device);
        let memory_types = p.memory_types[0..p.memory_type_count as usize].to_vec();
        let memory_heaps = p.memory_heaps[0..p.memory_heap_count as usize].to_vec();

        Allocator {
            memory_types,
            memory_heaps
        }
    }

    pub fn allocate(&self, vkd: &VkDevice1, info: &AllocationCreateInfo) -> Result<vk::DeviceMemory, AllocError>
    {
        let mut found_suitable_memory_type = false;
        // find a suitable memory type
        // first, look for mem types with required + preferred, then look again with only required
        // keep only memtypes that are compatible with the type bits.
        for (mt_index,mt) in self.memory_types.iter().enumerate().filter(|(_, mt)| mt.property_flags.subset(info.required_flags | info.preferred_flags))
            .chain(self.memory_types.iter().enumerate().filter(|(_, mt)| mt.property_flags.subset(info.required_flags) ))
            .filter(|&(mt_index,_)| (1 << (mt_index as u32)) & info.memory_type_bits != 0)
        {
            found_suitable_memory_type = true;
            debug!("alloc: allocating {} bytes in memory type {}", info.size, mt_index);
            //
            let vk_alloc_info = vk::MemoryAllocateInfo {
                allocation_size: info.size as vk::types::DeviceSize,
                memory_type_index: mt_index as u32,
                p_next: ptr::null(),
                s_type: vk::StructureType::MemoryAllocateInfo
            };

            let mem = unsafe {
                vkd.allocate_memory(&vk_alloc_info, None).map_err(|e| AllocError::Other(e))?
            };
            return Ok(mem)
        }

        if found_suitable_memory_type {
            Err(AllocError::Other(vk::Result::Success))
        } else {
            Err(AllocError::NoSuitableMemoryType)
        }
    }

}
