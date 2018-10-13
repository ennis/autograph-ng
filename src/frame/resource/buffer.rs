use frame::graph::TaskId;
use frame::resource::{BufferResource, Resource};
use frame::tasks::TaskOutput;

use sid_vec::{Id, IdVec};

pub struct BufferTag;
/// Identifies a buffer in the frame resource table.
pub type BufferId = Id<BufferTag, u32>;

/*
//--------------------------------------------------------------------------------------------------
impl<T: ?Sized, A> Resource for DeviceLocalBuffer<T, A>
{
    fn name(&self) -> &str {
        "unnamed buffer"
    }

    fn is_transient(&self) -> bool {
        false
    }

    fn is_allocated(&self) -> bool {
        true
    }
}

impl<T: ?Sized, A> BufferResource for DeviceLocalBuffer<T, A> where DeviceLocalBuffer<T, A>: BufferAccess
{
    fn byte_size(&self) -> u64 {
        BufferAccess::size(self) as u64
    }
}
*/
