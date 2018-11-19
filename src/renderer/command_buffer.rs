use std::ops::Range;

use crate::renderer::handles::*;
use crate::renderer::image::*;
use crate::renderer::sync::*;

struct Command {
    sort_key: u64,
    cmd: CommandInner,
}

enum CommandInner {
    PipelineBarrier {},
    AllocImage {
        image: ImageHandle,
    },
    AllocBuffer {
        buffer: BufferHandle,
    },
    DropImage {
        image: ImageHandle,
    },
    DropBuffer {
        buffer: BufferHandle,
    },
    SwapImages {
        a: ImageHandle,
        b: ImageHandle,
    },
    SwapBuffers {
        a: BufferHandle,
        b: BufferHandle,
    },
    ClearColorImage {
        image: ImageHandle,
        color: [f32; 4],
    },
    Present {
        image: ImageHandle,
        swapchain: SwapchainHandle,
    },
}

pub struct CommandBuffer {
    commands: Vec<Command>,
}

/// API exposed by command buffers.
/// Can build multiple command buffers concurrently in different threads.
impl CommandBuffer {
    pub(super) fn new() -> CommandBuffer {
        CommandBuffer {
            commands: Vec::new(),
        }
    }

    fn push_command(&mut self, sort_key: u64, cmd: CommandInner) {
        self.commands.push(Command { cmd, sort_key })
    }

    //----------------------------------------------------------------------------------------------
    // Manual sync

    /// Inserts an explicit pipeline barrier.
    pub fn pipeline_barrier(
        &mut self,
        sort_key: u64,
        src: PipelineStageFlags,
        dst: PipelineStageFlags,
        memory_barriers: &[MemoryBarrier],
    ) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Allocate

    /// Allocates or gets a temporary image to be used in this frame.
    /// (alloc_img <params>)
    pub fn alloc_image(&mut self, sort_key: u64, image: ImageHandle) {
        unimplemented!()
    }

    pub fn alloc_buffer(&mut self, sort_key: u64, buffer: BufferHandle) {
        unimplemented!()
    }

    /// Uploads data to a temporary buffer.
    pub fn upload(&mut self, name: Option<&str>, data: &[u8]) -> BufferHandle {
        unimplemented!()
    }

    /// Returns a reference to the named resource.
    pub fn create_image(&mut self) -> ImageHandle {
        unimplemented!()
    }

    /// Returns a reference to the named resource.
    pub fn create_buffer(&mut self) -> BufferHandle {
        unimplemented!()
    }

    /// Drops a temporary image.
    /// (drop_img <image>)
    pub fn drop_image(&mut self, sort_key: u64, image: ImageHandle) {
        unimplemented!()
    }

    /// Drops a temporary image.
    /// (drop_buf <image>)
    pub fn drop_buffer(&mut self, sort_key: u64, buffer: BufferHandle) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Swap

    /// Swaps two resources.
    /// (swap_img <image1> <image2>)
    pub fn swap_images(&mut self, img_a: ImageHandle, img_b: ImageHandle) {
        unimplemented!()
    }

    /// Swaps two resources.
    /// (swap_buf <buf1> <buf2>)
    pub fn swap_buffers(&mut self, buf_a: BufferHandle, buf_b: BufferHandle) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Copy

    /// Copy data between buffers.
    pub fn copy_buffer(
        &mut self,
        sort_key: u64,
        src: BufferHandle,
        dst: BufferHandle,
        src_range: Range<u64>,
        dst_range: Range<u64>,
    ) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Draw

    /// Presents the specified image to the swapchain.
    /// Might incur a copy / blit or format conversion if necessary.
    pub fn present(&mut self, sort_key: u64, image: ImageHandle, swapchain: SwapchainHandle) {
        self.push_command(sort_key, CommandInner::Present { image, swapchain })
    }
}
