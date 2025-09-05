//! Task-state segment

use super::PrivilegeLevel;
use super::segmentation::{SegmentAccess64, SegmentDescriptor64};

/// 64bit task state segment.
///
/// See Intel (R) 64 and IA-32 Architectures Software Developer’s Manual, Volume
/// 3A: System Programming Guide, Part 1 Figure 7-11.
#[repr(C, packed)]
pub struct TaskStateSegment {
    _res0: u32,
    pub rsp0: usize,
    pub rsp1: usize,
    pub rsp2: usize,
    _res1: u64,
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    _res2: u64,
    _res3: u16,
    pub io_map_base: u16,
}

impl TaskStateSegment {
    /// Create a empty TaskStateSegment.
    pub const fn empty() -> Self {
        Self {
            _res0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            _res1: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            _res2: 0,
            _res3: 0,
            io_map_base: 0,
        }
    }

    /// Fill the segment descriptor.
    pub fn fill_segment_descriptor(&'static self, desc: &mut SegmentDescriptor64) {
        *desc = SegmentDescriptor64::new(
            self as *const _ as usize as u64,
            core::mem::size_of::<Self>() as u64,
            SegmentAccess64::P | SegmentAccess64::T64A,
            PrivilegeLevel::Ring0,
        )
    }
}
