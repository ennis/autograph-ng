use crate::RendererBackend;
use bitflags::bitflags;

bitflags! {
    /// Used for manual synchronization.
    pub struct PipelineStageFlags: u32
    {
        const TOP_OF_PIPE_BIT = 0x0000_0001;
        const DRAW_INDIRECT_BIT = 0x0000_0002;
        const VERTEX_INPUT_BIT = 0x0000_0004;
        const VERTEX_SHADER_BIT = 0x0000_0008;
        const TESSELLATION_CONTROL_SHADER_BIT = 0x0000_0010;
        const TESSELLATION_EVALUATION_SHADER_BIT = 0x0000_0020;
        const GEOMETRY_SHADER_BIT = 0x0000_0040;
        const FRAGMENT_SHADER_BIT = 0x0000_0080;
        const EARLY_FRAGMENT_TESTS_BIT = 0x0000_0100;
        const LATE_FRAGMENT_TESTS_BIT = 0x0000_0200;
        const COLOR_ATTACHMENT_OUTPUT_BIT = 0x0000_0400;
        const COMPUTE_SHADER_BIT = 0x0000_0800;
        const TRANSFER_BIT = 0x0000_1000;
        const BOTTOM_OF_PIPE_BIT = 0x0000_2000;
        const HOST_BIT = 0x0000_4000;
        const ALL_GRAPHICS_BIT = 0x0000_8000;
        const ALL_COMMANDS_BIT = 0x0001_0000;
        const TRANSFORM_FEEDBACK_BIT_EXT = 0x0100_0000;
        const CONDITIONAL_RENDERING_BIT_EXT = 0x0004_0000;
        const COMMAND_PROCESS_BIT_NVX = 0x0002_0000;
        const SHADING_RATE_IMAGE_BIT_NV = 0x0040_0000;
        const RAY_TRACING_SHADER_BIT_NV = 0x0020_0000;
        const ACCELERATION_STRUCTURE_BUILD_BIT_NV = 0x0200_0000;
        const TASK_SHADER_BIT_NV = 0x0008_0000;
        const MESH_SHADER_BIT_NV = 0x0010_0000;
    }
}

bitflags! {
    /// Used for manual synchronization.
    pub struct AccessFlags: u32
    {
        const INDIRECT_COMMAND_READ_BIT = 0x0000_0001;
        const INDEX_READ_BIT = 0x0000_0002;
        const VERTEX_ATTRIBUTE_READ_BIT = 0x0000_0004;
        const UNIFORM_READ_BIT = 0x0000_0008;
        const INPUT_ATTACHMENT_READ_BIT = 0x0000_0010;
        const SHADER_READ_BIT = 0x0000_0020;
        const SHADER_WRITE_BIT = 0x0000_0040;
        const COLOR_ATTACHMENT_READ_BIT = 0x0000_0080;
        const COLOR_ATTACHMENT_WRITE_BIT = 0x0000_0100;
        const DEPTH_STENCIL_ATTACHMENT_READ_BIT = 0x0000_0200;
        const DEPTH_STENCIL_ATTACHMENT_WRITE_BIT = 0x0000_0400;
        const TRANSFER_READ_BIT = 0x0000_0800;
        const TRANSFER_WRITE_BIT = 0x0000_1000;
        const HOST_READ_BIT = 0x0000_2000;
        const HOST_WRITE_BIT = 0x0000_4000;
        const MEMORY_READ_BIT = 0x0000_8000;
        const MEMORY_WRITE_BIT = 0x0001_0000;
        const TRANSFORM_FEEDBACK_WRITE_BIT_EXT = 0x0200_0000;
        const TRANSFORM_FEEDBACK_COUNTER_READ_BIT_EXT = 0x0400_0000;
        const TRANSFORM_FEEDBACK_COUNTER_WRITE_BIT_EXT = 0x0800_0000;
        const CONDITIONAL_RENDERING_READ_BIT_EXT = 0x0010_0000;
        const COMMAND_PROCESS_READ_BIT_NVX = 0x0002_0000;
        const COMMAND_PROCESS_WRITE_BIT_NVX = 0x0004_0000;
        const COLOR_ATTACHMENT_READ_NONCOHERENT_BIT_EXT = 0x0008_0000;
        const SHADING_RATE_IMAGE_READ_BIT_NV = 0x0080_0000;
        const ACCELERATION_STRUCTURE_READ_BIT_NV = 0x0020_0000;
        const ACCELERATION_STRUCTURE_WRITE_BIT_NV = 0x0040_0000;
    }
}

#[derive(Clone, Debug)]
pub enum MemoryBarrier<'a, R: RendererBackend> {
    Image {
        handle: &'a R::Image,
        src_access_mask: AccessFlags,
        dst_access_mask: AccessFlags,
    },
    Buffer {
        handle: &'a R::Buffer,
        src_access_mask: AccessFlags,
        dst_access_mask: AccessFlags,
    },
}
