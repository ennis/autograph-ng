use std::mem;
use std::ops::Range;

use crate::renderer::sync::*;
use crate::renderer::{
    shader_interface::{PipelineInterface, PipelineInterfaceVisitor},
    RendererBackend, ScissorRect, Viewport,
};

pub struct Command<'a, R: RendererBackend> {
    pub sort_key: u64,
    pub cmd: CommandInner<'a, R>,
}

// Explicit clone impl because of #26925
impl<'a, R: RendererBackend> Clone for Command<'a, R> {
    fn clone(&self) -> Self {
        Command {
            cmd: self.cmd.clone(),
            sort_key: self.sort_key,
        }
    }
}

/*
pub struct CmdSetVertexBuffers<'a, R: RendererBackend> {
    count: usize,
    buffers: [&'a R::Buffer]
}*/

// command header(with sort key), followed by subcommands (state-change commands)

pub enum DrawCommand {
    DrawArrays {
        first: u32,
        count: u32,
    },
    DrawIndexed {
        first: u32,
        count: u32,
        base_vertex: u32,
    },
}

pub enum CommandInner<'a, R: RendererBackend> {
    // MAIN COMMANDS -------------------------------------------------------------------------------
    PipelineBarrier {},
    ClearImageFloat {
        image: &'a R::Image,
        color: [f32; 4],
    },
    ClearDepthStencilImage {
        image: &'a R::Image,
        depth: f32,
        stencil: Option<u8>,
    },
    Present {
        image: &'a R::Image,
        swapchain: &'a R::Swapchain,
    },
    DrawHeader {
        pipeline: &'a R::GraphicsPipeline,
    },

    // STATE CHANGE COMMANDS -----------------------------------------------------------------------
    SetDescriptorSets {
        descriptor_sets: Vec<&'a R::DescriptorSet>,
    },
    SetFramebuffer {
        framebuffer: &'a R::Framebuffer,
    },
    SetVertexBuffers {
        vertex_buffers: Vec<&'a R::Buffer>,
    },
    SetIndexBuffer {
        index_buffer: Option<&'a R::Buffer>,
    },
    SetScissors {
        first: u32,
        scissors: Vec<ScissorRect>,
    },
    SetAllScissors {
        scissor: ScissorRect,
    },
    SetViewports {
        first: u32,
        viewports: Vec<Viewport>,
    },
    SetAllViewports {
        viewport: Viewport,
    },
    Draw {
        draw: DrawCommand,
    },
}


// Explicit clone impl because of #26925
impl<'a, R: RendererBackend> Clone for CommandInner<'a, R> {
    fn clone(&self) -> Self {
        // I really don't want to match all variants just to copy bits around.
        unsafe { mem::transmute_copy(self) }
    }
}

pub struct CommandBuffer<'a, R: RendererBackend> {
    commands: Vec<Command<'a, R>>,
}

/// API exposed by command buffers.
/// Can build multiple command buffers concurrently in different threads.
impl<'a, R: RendererBackend> CommandBuffer<'a, R> {
    pub(super) fn new() -> CommandBuffer<'a, R> {
        CommandBuffer {
            commands: Vec::new(),
        }
    }

    fn push_command(&mut self, sort_key: u64, cmd: CommandInner<'a, R>) {
        self.commands.push(Command { cmd, sort_key })
    }

    // fn self.push_header_command(sort_key)
    // fn self.push_trailing_command()

    pub fn iter(&self) -> impl Iterator<Item = &Command<'a, R>> {
        self.commands.iter()
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
    pub fn alloc_image(&mut self, sort_key: u64, image: &'a R::Image) {
        unimplemented!()
    }

    pub fn alloc_buffer(&mut self, sort_key: u64, buffer: &'a R::Buffer) {
        unimplemented!()
    }

    /*/// Uploads data to a temporary buffer.
    pub fn upload(&mut self, name: Option<&str>, data: &[u8]) -> &'a R::Buffer {
        unimplemented!()
    }*/

    /*
    /// Returns a reference to the named resource.
    pub fn create_image(&mut self) -> R::Image {
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
    }*/

    //----------------------------------------------------------------------------------------------
    // Swap

    /// Swaps two resources.
    /// (swap_img <image1> <image2>)
    pub fn swap_images(&mut self, sort_key: u64, img_a: &'a R::Image, img_b: &'a R::Image) {
        unimplemented!()
    }

    /// Swaps two resources.
    /// (swap_buf <buf1> <buf2>)
    pub fn swap_buffers(&mut self, sort_key: u64, buf_a: &'a R::Buffer, buf_b: &'a R::Buffer) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Copy

    /// Copy data between buffers.
    pub fn copy_buffer(
        &mut self,
        sort_key: u64,
        src: &'a R::Buffer,
        dst: &'a R::Buffer,
        src_range: Range<u64>,
        dst_range: Range<u64>,
    ) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Clear

    /// Clears an image.
    pub fn clear_image(&mut self, sort_key: u64, image: &'a R::Image, color: &[f32; 4]) {
        self.push_command(
            sort_key,
            CommandInner::ClearImageFloat {
                image,
                color: *color,
            },
        )
    }

    /// Clears an image.
    pub fn clear_depth_stencil_image(
        &mut self,
        sort_key: u64,
        image: &'a R::Image,
        depth: f32,
        stencil: Option<u8>,
    ) {
        self.push_command(
            sort_key,
            CommandInner::ClearDepthStencilImage {
                image,
                depth,
                stencil,
            },
        )
    }

    //----------------------------------------------------------------------------------------------
    // Draw

    fn set_descriptor_sets(&mut self, sort_key: u64, descriptor_sets: &[&'a R::DescriptorSet]) {
        self.push_command(
            sort_key,
            CommandInner::SetDescriptorSets {
                descriptor_sets: descriptor_sets.to_vec(),
            },
        )
    }

    fn set_framebuffer(&mut self, sort_key: u64, framebuffer: &'a R::Framebuffer) {
        self.push_command(sort_key, CommandInner::SetFramebuffer { framebuffer })
    }

    fn set_vertex_buffers(&mut self, sort_key: u64, vertex_buffers: &[&'a R::Buffer]) {
        self.push_command(
            sort_key,
            CommandInner::SetVertexBuffers {
                vertex_buffers: vertex_buffers.to_vec(),
            },
        )
    }

    fn set_index_buffer(&mut self, sort_key: u64, index_buffer: Option<&'a R::Buffer>) {
        self.push_command(sort_key, CommandInner::SetIndexBuffer { index_buffer })
    }

    fn set_viewports(&mut self, sort_key: u64, first: u32, viewports: &[Viewport]) {
        self.push_command(
            sort_key,
            CommandInner::SetViewports {
                first,
                viewports: viewports.to_vec(),
            },
        )
    }

    fn set_all_viewports(&mut self, sort_key: u64, viewport: &Viewport) {
        self.push_command(
            sort_key,
            CommandInner::SetAllViewports {
                viewport: *viewport,
            },
        )
    }

    fn set_scissors(&mut self, sort_key: u64, first: u32, scissors: &[ScissorRect]) {
        self.push_command(
            sort_key,
            CommandInner::SetScissors {
                first,
                scissors: scissors.to_vec(),
            },
        )
    }

    fn set_all_scissors(&mut self, sort_key: u64, scissor: &ScissorRect) {
        self.push_command(sort_key, CommandInner::SetAllScissors { scissor: *scissor })
    }

    // need:
    pub fn draw<PI: PipelineInterface<'a, R>>(
        &mut self,
        sort_key: u64,
        pipeline: &'a R::GraphicsPipeline,
        interface: &PI,
    ) {
        self.push_command(sort_key, CommandInner::DrawHeader { pipeline });

        struct Visitor<'a, 'b, R: RendererBackend> {
            sort_key: u64,
            cmdbuf: &'b mut CommandBuffer<'a, R>,
        }

        impl<'a, 'b, R: RendererBackend> PipelineInterfaceVisitor<'a, R> for Visitor<'a, 'b, R> {
            fn visit_descriptor_sets(&mut self, descriptor_sets: &[&'a R::DescriptorSet]) {
                self.cmdbuf
                    .set_descriptor_sets(self.sort_key, descriptor_sets);
            }

            fn visit_vertex_buffers(&mut self, vertex_buffers: &[&'a R::Buffer]) {
                self.cmdbuf
                    .set_vertex_buffers(self.sort_key, vertex_buffers);
            }

            fn visit_index_buffer(&mut self, index_buffer: &'a R::Buffer) {
                self.cmdbuf
                    .set_index_buffer(self.sort_key, Some(index_buffer));
            }

            fn visit_framebuffer(&mut self, framebuffer: &'a R::Framebuffer) {
                self.cmdbuf.set_framebuffer(self.sort_key, framebuffer);
            }

            fn visit_dynamic_viewports(&mut self, first: u32, viewports: &[Viewport]) {
                self.cmdbuf.set_viewports(self.sort_key, first, viewports);
            }

            fn visit_dynamic_viewport_all(&mut self, viewport: &Viewport) {
                self.cmdbuf.set_all_viewports(self.sort_key, viewport);
            }

            fn visit_dynamic_scissors(&mut self, first: u32, scissors: &[ScissorRect]) {
                self.cmdbuf.set_scissors(self.sort_key, first, scissors);
            }

            fn visit_dynamic_scissor_all(&mut self, scissor: &ScissorRect) {
                self.cmdbuf.set_all_scissors(self.sort_key, scissor);
            }
        }

        let mut v = Visitor {
            sort_key,
            cmdbuf: self,
        };

        interface.do_visit(&mut v);
    }

    //----------------------------------------------------------------------------------------------
    // Present

    /// Presents the specified image to the swapchain.
    /// Might incur a copy / blit or format conversion if necessary.
    pub fn present(&mut self, sort_key: u64, image: &'a R::Image, swapchain: &'a R::Swapchain) {
        self.push_command(sort_key, CommandInner::Present { image, swapchain })
    }
}

/// TODO optimize (radix sort, dense command buffer layout, separate index map)
pub fn sort_command_buffers<'a, R: RendererBackend>(
    cmdbufs: Vec<CommandBuffer<'a, R>>,
) -> Vec<Command<'a, R>> {
    let mut fused = Vec::new();
    //let mut sortkeys = Vec::new();
    //let mut i: usize = 0;
    for cmdbuf in cmdbufs.iter() {
        for cmd in cmdbuf.commands.iter() {
            fused.push(cmd.clone());
            //sortkeys.push(cmd.sort_key);
        }
    }

    fused.sort_by(|cmd_a, cmd_b| cmd_a.sort_key.cmp(&cmd_b.sort_key));
    fused
}
