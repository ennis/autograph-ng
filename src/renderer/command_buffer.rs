
struct Command
{
    sort_key: u64,
    cmd: CommandInner
}

enum CommandInner
{
    PipelineBarrier {},
    AllocImage {},
    AllocBuffer {},
    DropImage {},
    DropBuffer {},
    SwapImages {},
    SwapBuffers {},
}

pub struct CommandBuffer<'a>
{
    renderer: &'a Renderer,
    commands: Vec<Command>
}

/// API exposed by command buffers.
/// Can build multiple command buffers concurrently in different threads.
impl CommandBuffer
{
    //----------------------------------------------------------------------------------------------
    // Manual sync

    /// Inserts an explicit pipeline barrier.
    fn pipeline_barrier(
        &self,
        sort_key: u64,
        src: PipelineBarrierStages,
        dst: PipelineBarrierStages,
        memory_barriers: &[MemoryBarrier])
    {}

    //----------------------------------------------------------------------------------------------
    // Allocate

    /// Allocates or gets a temporary image to be used in this frame.
    /// (alloc_img <params>)
    fn alloc_image(
        &self,
        sort_key: u64,
        name: Option<&str>,
        dimensions: Dimensions,
        mipcount: MipmapsCount,
        samples: u32,
        format: Format,
        usage: ImageUsage) -> ImageHandle
    {

    }

    fn alloc_buffer(
        &self,
        sort_key: u64,
        name: Option<&str>,
        size: u64,
        memory: MemoryType) -> BufferHandle;

    /// Uploads data to a temporary buffer.
    fn upload(&self, name: Option<&str>, data: &[u8]) -> BufferHandle;

    /// Returns a reference to the named resource.
    fn create_image(&self) -> ImageHandle
    {}

    /// Returns a reference to the named resource.
    fn create_buffer(&self) -> BufferHandle
    {}

    /// Drops a temporary image.
    /// (drop_img <image>)
    fn drop_image(&self, sort_key: u64, image: ImageHandle);

    /// Drops a temporary image.
    /// (drop_buf <image>)
    fn drop_buffer(&self, sort_key: u64, buffer: BufferHandle);

    //----------------------------------------------------------------------------------------------
    // Swap

    /// Swaps two resources.
    /// (swap_img <image1> <image2>)
    fn swap_images(&self, img_a: ImageHandle, img_b: ImageHandle);

    /// Swaps two resources.
    /// (swap_buf <buf1> <buf2>)
    fn swap_buffers(&self, buf_a: BufferHandle, buf_b: BufferHandle);

    //----------------------------------------------------------------------------------------------
    // Copy

    /// Copy data between buffers.
    fn copy_buffer(&self,
                   sort_key: u64,
                   src: BufferHandle,
                   dst: BufferHandle,
                   src_range: Range<u64>,
                   dst_range: Range<u64>);

    //----------------------------------------------------------------------------------------------
    // Draw

    /// Presents the specified image to the swapchain.
    /// Might incur a copy / blit or format conversion if necessary.
    fn present(&self, image: ImageHandle, swapchain: SwapchainHandle);

    /*/// Executes a secondary command buffer.
    fn execute(&self, cmdbuf: CommandBufferHandle);*/

    /// Stops recording commands and submit this command buffer.
    /// Recording other commands is invalid after that.
    fn finish(&self);
}
