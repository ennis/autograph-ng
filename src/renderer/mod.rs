mod image;
mod format;
mod handles;

pub use self::image::*;
pub use self::format::*;
pub use self::handles::*;

#[derive(Copy, Clone, Debug)]
pub enum MemoryType
{
    DeviceLocal,
    HostUpload,
    HostReadback
}

pub enum Queue
{
    Graphics,
    Compute,
    Transfer,
}

bitflags! {
    /// Used for manual synchronization.
    pub struct PipelineStageFlags: u32
    {
        const TOP_OF_PIPE_BIT = 0x00000001;
        const DRAW_INDIRECT_BIT = 0x00000002;
        const VERTEX_INPUT_BIT = 0x00000004;
        const VERTEX_SHADER_BIT = 0x00000008;
        const TESSELLATION_CONTROL_SHADER_BIT = 0x00000010;
        const TESSELLATION_EVALUATION_SHADER_BIT = 0x00000020;
        const GEOMETRY_SHADER_BIT = 0x00000040;
        const FRAGMENT_SHADER_BIT = 0x00000080;
        const EARLY_FRAGMENT_TESTS_BIT = 0x00000100;
        const LATE_FRAGMENT_TESTS_BIT = 0x00000200;
        const COLOR_ATTACHMENT_OUTPUT_BIT = 0x00000400;
        const COMPUTE_SHADER_BIT = 0x00000800;
        const TRANSFER_BIT = 0x00001000;
        const BOTTOM_OF_PIPE_BIT = 0x00002000;
        const HOST_BIT = 0x00004000;
        const ALL_GRAPHICS_BIT = 0x00008000;
        const ALL_COMMANDS_BIT = 0x00010000;
        const TRANSFORM_FEEDBACK_BIT_EXT = 0x01000000;
        const CONDITIONAL_RENDERING_BIT_EXT = 0x00040000;
        const COMMAND_PROCESS_BIT_NVX = 0x00020000;
        const SHADING_RATE_IMAGE_BIT_NV = 0x00400000;
        const RAY_TRACING_SHADER_BIT_NV = 0x00200000;
        const ACCELERATION_STRUCTURE_BUILD_BIT_NV = 0x02000000;
        const TASK_SHADER_BIT_NV = 0x00080000;
        const MESH_SHADER_BIT_NV = 0x00100000;
    }
}

bitflags! {
    /// Used for manual synchronization.
    pub struct AccessFlags: u32
    {
        const INDIRECT_COMMAND_READ_BIT = 0x00000001;
        const INDEX_READ_BIT = 0x00000002;
        const VERTEX_ATTRIBUTE_READ_BIT = 0x00000004;
        const UNIFORM_READ_BIT = 0x00000008;
        const INPUT_ATTACHMENT_READ_BIT = 0x00000010;
        const SHADER_READ_BIT = 0x00000020;
        const SHADER_WRITE_BIT = 0x00000040;
        const COLOR_ATTACHMENT_READ_BIT = 0x00000080;
        const COLOR_ATTACHMENT_WRITE_BIT = 0x00000100;
        const DEPTH_STENCIL_ATTACHMENT_READ_BIT = 0x00000200;
        const DEPTH_STENCIL_ATTACHMENT_WRITE_BIT = 0x00000400;
        const TRANSFER_READ_BIT = 0x00000800;
        const TRANSFER_WRITE_BIT = 0x00001000;
        const HOST_READ_BIT = 0x00002000;
        const HOST_WRITE_BIT = 0x00004000;
        const MEMORY_READ_BIT = 0x00008000;
        const MEMORY_WRITE_BIT = 0x00010000;
        const TRANSFORM_FEEDBACK_WRITE_BIT_EXT = 0x02000000;
        const TRANSFORM_FEEDBACK_COUNTER_READ_BIT_EXT = 0x04000000;
        const TRANSFORM_FEEDBACK_COUNTER_WRITE_BIT_EXT = 0x08000000;
        const CONDITIONAL_RENDERING_READ_BIT_EXT = 0x00100000;
        const COMMAND_PROCESS_READ_BIT_NVX = 0x00020000;
        const COMMAND_PROCESS_WRITE_BIT_NVX = 0x00040000;
        const COLOR_ATTACHMENT_READ_NONCOHERENT_BIT_EXT = 0x00080000;
        const SHADING_RATE_IMAGE_READ_BIT_NV = 0x00800000;
        const ACCELERATION_STRUCTURE_READ_BIT_NV = 0x00200000;
        const ACCELERATION_STRUCTURE_WRITE_BIT_NV = 0x00400000;
    }
}

#[derive(Clone, Debug)]
pub enum MemoryBarrier {
    Image {
        handle: ImageHandle,
        src_access_mask: AccessFlags,
        dst_access_mask: AccessFlags,
    },
    Buffer {
        handle: BufferHandle,
        src_access_mask: AccessFlags,
        dst_access_mask: AccessFlags,
    }
}

pub trait Renderer
{
    /// Creates a swapchain.
    fn create_swapchain(&self) -> SwapchainHandle;

    /// Creates an image.
    fn create_image(
        &self,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        format: Format,
        usage: ImageUsage) -> ImageHandle;

    /// Uploads data to a transient pool.
    /// The buffer becomes invalid as soon as out of the current frame.
    /// The buffer can be used as uniform input to pipelines.
    fn upload_transient(&self, data: &[u8]) -> BufferHandle;

    /// Destroys an image handle. The actual image is destroyed when
    /// it is not in use anymore by the GPU.
    fn destroy_image(&self, image: ImageHandle);

    /// Creates a GPU (device local) buffer.
    /// This function only creates a handle (name) and description of the buffer.
    /// For the memory to be allocated, it has to be initialized by a command in a command buffer.
    /// This function is thread-safe.
    fn create_buffer(
        &self,
        size: u64,
    ) -> BufferHandle;

    /// Destroys a GPU buffer. The actual buffer is destroyed when
    /// it is not in use anymore by the GPU.
    /// TODO: do it in a command buffer?
    fn destroy_buffer(&self, buffer: BufferHandle);

    /// Submits a command buffer.
    fn submit_command_buffer(&self, cmdbuf: CommandBuffer);

    /// Signals the end of the current frame, and starts another.
    fn end_frame(&self);
}

