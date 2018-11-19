use std::ops::Range;

pub fn align_offset(size: u64, align: u64, space: Range<u64>) -> Option<u64> {
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
