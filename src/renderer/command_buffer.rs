use std::ops::Range;

use crate::renderer::RendererBackend;
use crate::renderer::image::*;
use crate::renderer::sync::*;

struct Command<R: RendererBackend> {
    sort_key: u64,
    cmd: CommandInner<R>,
}

enum CommandInner<R: RendererBackend> {
    PipelineBarrier {},
    AllocImage {
        image: R::ImageHandle,
    },
    AllocBuffer {
        buffer: R::BufferHandle,
    },
    DropImage {
        image: R::ImageHandle,
    },
    DropBuffer {
        buffer: R::BufferHandle,
    },
    SwapImages {
        a: R::ImageHandle,
        b: R::ImageHandle,
    },
    SwapBuffers {
        a: R::BufferHandle,
        b: R::BufferHandle,
    },
    ClearColorImage {
        image: R::ImageHandle,
        color: [f32; 4],
    },
    Present {
        image: R::ImageHandle,
        swapchain: R::SwapchainHandle,
    },
}

pub struct CommandBuffer<R: RendererBackend> {
    commands: Vec<Command<R>>,
}

/// API exposed by command buffers.
/// Can build multiple command buffers concurrently in different threads.
impl<R: RendererBackend> CommandBuffer<R> {
    pub(super) fn new() -> CommandBuffer<R> {
        CommandBuffer {
            commands: Vec::new(),
        }
    }

    fn push_command(&mut self, sort_key: u64, cmd: CommandInner<R>) {
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
        memory_barriers: &[MemoryBarrier<R>],
    ) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Allocate

    /// Allocates or gets a temporary image to be used in this frame.
    /// (alloc_img <params>)
    pub fn alloc_image(&mut self, sort_key: u64, image: R::ImageHandle) {
        unimplemented!()
    }

    pub fn alloc_buffer(&mut self, sort_key: u64, buffer: R::BufferHandle) {
        unimplemented!()
    }

    /// Uploads data to a temporary buffer.
    pub fn upload(&mut self, name: Option<&str>, data: &[u8]) -> R::BufferHandle {
        unimplemented!()
    }

    /// Returns a reference to the named resource.
    pub fn create_image(&mut self) -> R::ImageHandle {
        unimplemented!()
    }

    /// Returns a reference to the named resource.
    pub fn create_buffer(&mut self) -> R::BufferHandle {
        unimplemented!()
    }

    /// Drops a temporary image.
    /// (drop_img <image>)
    pub fn drop_image(&mut self, sort_key: u64, image: R::ImageHandle) {
        unimplemented!()
    }

    /// Drops a temporary image.
    /// (drop_buf <image>)
    pub fn drop_buffer(&mut self, sort_key: u64, buffer: R::BufferHandle) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Swap

    /// Swaps two resources.
    /// (swap_img <image1> <image2>)
    pub fn swap_images(&mut self, sort_key: u64, img_a: R::ImageHandle, img_b: R::ImageHandle) {
        unimplemented!()
    }

    /// Swaps two resources.
    /// (swap_buf <buf1> <buf2>)
    pub fn swap_buffers(&mut self, sort_key: u64, buf_a: R::BufferHandle, buf_b: R::BufferHandle) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Copy

    /// Copy data between buffers.
    pub fn copy_buffer(
        &mut self,
        sort_key: u64,
        src: R::BufferHandle,
        dst: R::BufferHandle,
        src_range: Range<u64>,
        dst_range: Range<u64>,
    ) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Draw

    /// Presents the specified image to the swapchain.
    /// Might incur a copy / blit or format conversion if necessary.
    pub fn present(&mut self, sort_key: u64, image: R::ImageHandle, swapchain: R::SwapchainHandle) {
        self.push_command(sort_key, CommandInner::Present { image, swapchain })
    }
}
