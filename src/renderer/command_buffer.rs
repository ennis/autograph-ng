use std::mem;
use std::ops::Range;

use crate::renderer::sync::*;
use crate::renderer::{
    shader_interface::{PipelineInterface, PipelineInterfaceVisitor},
    Buffer, BufferTypeless, DescriptorSet, DescriptorSetLayout, Framebuffer, GraphicsPipeline,
    Image, IndexType, RendererBackend, ScissorRect, Swapchain, Viewport,
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

#[derive(Copy, Clone, Debug)]
pub struct DrawParams {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct DrawIndexedParams {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

pub enum CommandInner<'a, R: RendererBackend> {
    // MAIN (LEAD-IN) COMMANDS ---------------------------------------------------------------------
    PipelineBarrier {},
    ClearImageFloat {
        image: Image<'a, R>,
        color: [f32; 4],
    },
    ClearDepthStencilImage {
        image: Image<'a, R>,
        depth: f32,
        stencil: Option<u8>,
    },
    Present {
        image: Image<'a, R>,
        swapchain: Swapchain<'a, R>,
    },
    DrawHeader {
        pipeline: GraphicsPipeline<'a, R>,
    },

    // STATE CHANGE COMMANDS -----------------------------------------------------------------------
    SetDescriptorSets {
        descriptor_sets: Vec<DescriptorSet<'a, R>>,
    },
    SetFramebuffer {
        framebuffer: Framebuffer<'a, R>,
    },
    SetVertexBuffers {
        vertex_buffers: Vec<BufferTypeless<'a, R>>,
    },
    SetIndexBuffer {
        index_buffer: BufferTypeless<'a, R>,
        offset: usize,
        ty: IndexType,
    },
    SetScissors {
        //first: u32,
        scissors: Vec<ScissorRect>,
    },
    SetViewports {
        //first: u32,
        viewports: Vec<Viewport>,
    },

    // DRAW (LEAD-OUT) COMMANDS --------------------------------------------------------------------
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    },
}

// Explicit clone impl because of #26925
impl<'a, R: RendererBackend> Clone for CommandInner<'a, R> {
    fn clone(&self) -> Self {
        // I really don't want to match all variants just to copy bits around.
        // unsafe { mem::transmute_copy(self) }
        // yeah, let's not do that after all...
        // I inadvertantly put a member with a destructor in a variant and chased a use-after-free
        // for hours.
        match *self {
            CommandInner::PipelineBarrier {} => CommandInner::PipelineBarrier {},
            CommandInner::ClearImageFloat { image, color } => {
                CommandInner::ClearImageFloat { image, color }
            }
            CommandInner::ClearDepthStencilImage {
                image,
                depth,
                stencil,
            } => CommandInner::ClearDepthStencilImage {
                image,
                depth,
                stencil,
            },
            CommandInner::Present { image, swapchain } => {
                CommandInner::Present { image, swapchain }
            }
            CommandInner::DrawHeader { pipeline } => CommandInner::DrawHeader { pipeline },

            CommandInner::SetDescriptorSets {
                ref descriptor_sets,
            } => CommandInner::SetDescriptorSets {
                descriptor_sets: descriptor_sets.clone(),
            },
            CommandInner::SetFramebuffer { framebuffer } => {
                CommandInner::SetFramebuffer { framebuffer }
            }
            CommandInner::SetVertexBuffers { ref vertex_buffers } => {
                CommandInner::SetVertexBuffers {
                    vertex_buffers: vertex_buffers.clone(),
                }
            }
            CommandInner::SetIndexBuffer {
                index_buffer,
                offset,
                ty,
            } => CommandInner::SetIndexBuffer {
                index_buffer,
                offset,
                ty,
            },
            CommandInner::SetScissors { ref scissors } => CommandInner::SetScissors {
                scissors: scissors.clone(),
            },
            //CommandInner::SetAllScissors { scissor } => CommandInner::SetAllScissors { scissor },
            CommandInner::SetViewports { ref viewports } => CommandInner::SetViewports {
                viewports: viewports.clone(),
            },
            // CommandInner::SetAllViewports { viewport } => {
            //    CommandInner::SetAllViewports { viewport }
            //}
            CommandInner::Draw {
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            } => CommandInner::Draw {
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            },

            CommandInner::DrawIndexed {
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            } => CommandInner::DrawIndexed {
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            },
        }
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
    // Copy

    /// Copy data between buffers.
    pub fn copy_buffer(
        &mut self,
        sort_key: u64,
        src: BufferTypeless<'a, R>,
        dst: BufferTypeless<'a, R>,
        src_range: Range<u64>,
        dst_range: Range<u64>,
    ) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Clear

    /// Clears an image.
    pub fn clear_image(&mut self, sort_key: u64, image: Image<'a, R>, color: &[f32; 4]) {
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
        image: Image<'a, R>,
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

    fn set_descriptor_sets(&mut self, sort_key: u64, descriptor_sets: &[DescriptorSet<'a, R>]) {
        self.push_command(
            sort_key,
            CommandInner::SetDescriptorSets {
                descriptor_sets: descriptor_sets.to_vec(),
            },
        )
    }

    fn set_framebuffer(&mut self, sort_key: u64, framebuffer: Framebuffer<'a, R>) {
        self.push_command(sort_key, CommandInner::SetFramebuffer { framebuffer })
    }

    fn set_vertex_buffers(&mut self, sort_key: u64, vertex_buffers: &[BufferTypeless<'a, R>]) {
        self.push_command(
            sort_key,
            CommandInner::SetVertexBuffers {
                vertex_buffers: vertex_buffers.to_vec(),
            },
        )
    }

    fn set_index_buffer(
        &mut self,
        sort_key: u64,
        index_buffer: BufferTypeless<'a, R>,
        offset: usize,
        ty: IndexType,
    ) {
        self.push_command(
            sort_key,
            CommandInner::SetIndexBuffer {
                index_buffer,
                offset,
                ty,
            },
        )
    }

    fn set_viewports(&mut self, sort_key: u64, viewports: &[Viewport]) {
        self.push_command(
            sort_key,
            CommandInner::SetViewports {
                viewports: viewports.to_vec(),
            },
        )
    }

    /*fn set_all_viewports(&mut self, sort_key: u64, viewport: &Viewport) {
        self.push_command(
            sort_key,
            CommandInner::SetAllViewports {
                viewport: *viewport,
            },
        )
    }*/

    fn set_scissors(&mut self, sort_key: u64, scissors: &[ScissorRect]) {
        self.push_command(
            sort_key,
            CommandInner::SetScissors {
                scissors: scissors.to_vec(),
            },
        )
    }

    /* fn set_all_scissors(&mut self, sort_key: u64, scissor: &ScissorRect) {
        self.push_command(sort_key, CommandInner::SetAllScissors { scissor: *scissor })
    }*/

    fn bind_pipeline_interface<PI: PipelineInterface<'a, R>>(
        &mut self,
        sort_key: u64,
        pipeline: GraphicsPipeline<'a, R>,
        interface: &PI,
    ) {
        self.push_command(sort_key, CommandInner::DrawHeader { pipeline });

        struct Visitor<'a, 'b, R: RendererBackend> {
            sort_key: u64,
            cmdbuf: &'b mut CommandBuffer<'a, R>,
        }

        impl<'a, 'b, R: RendererBackend> PipelineInterfaceVisitor<'a, R> for Visitor<'a, 'b, R> {
            fn visit_descriptor_sets(&mut self, descriptor_sets: &[DescriptorSet<'a, R>]) {
                self.cmdbuf
                    .set_descriptor_sets(self.sort_key, descriptor_sets);
            }

            fn visit_vertex_buffers(&mut self, vertex_buffers: &[BufferTypeless<'a, R>]) {
                self.cmdbuf
                    .set_vertex_buffers(self.sort_key, vertex_buffers);
            }

            fn visit_index_buffer(
                &mut self,
                index_buffer: BufferTypeless<'a, R>,
                offset: usize,
                ty: IndexType,
            ) {
                self.cmdbuf
                    .set_index_buffer(self.sort_key, index_buffer, offset, ty);
            }

            fn visit_framebuffer(&mut self, framebuffer: Framebuffer<'a, R>) {
                self.cmdbuf.set_framebuffer(self.sort_key, framebuffer);
            }

            fn visit_dynamic_viewports(&mut self, viewports: &[Viewport]) {
                self.cmdbuf.set_viewports(self.sort_key, viewports);
            }

            /*fn visit_dynamic_viewport_all(&mut self, viewport: &Viewport) {
                self.cmdbuf.set_all_viewports(self.sort_key, viewport);
            }*/

            fn visit_dynamic_scissors(&mut self, scissors: &[ScissorRect]) {
                self.cmdbuf.set_scissors(self.sort_key, scissors);
            }

            /*fn visit_dynamic_scissor_all(&mut self, scissor: &ScissorRect) {
                self.cmdbuf.set_all_scissors(self.sort_key, scissor);
            }*/
        }

        let mut v = Visitor {
            sort_key,
            cmdbuf: self,
        };

        interface.do_visit(&mut v);
    }

    pub fn draw<PI: PipelineInterface<'a, R>>(
        &mut self,
        sort_key: u64,
        pipeline: GraphicsPipeline<'a, R>,
        interface: &PI,
        params: DrawParams,
    ) {
        self.bind_pipeline_interface(sort_key, pipeline, interface);
        self.push_command(
            sort_key,
            CommandInner::Draw {
                vertex_count: params.vertex_count,
                instance_count: params.instance_count,
                first_vertex: params.first_vertex,
                first_instance: params.first_instance,
            },
        );
    }

    pub fn draw_indexed<PI: PipelineInterface<'a, R>>(
        &mut self,
        sort_key: u64,
        pipeline: GraphicsPipeline<'a, R>,
        interface: &PI,
        params: DrawIndexedParams,
    ) {
        self.bind_pipeline_interface(sort_key, pipeline, interface);
        self.push_command(
            sort_key,
            CommandInner::DrawIndexed {
                index_count: params.index_count,
                instance_count: params.instance_count,
                first_index: params.first_index,
                vertex_offset: params.vertex_offset,
                first_instance: params.first_instance,
            },
        );
    }

    //----------------------------------------------------------------------------------------------
    // Present

    /// Presents the specified image to the swapchain.
    /// Might incur a copy / blit or format conversion if necessary.
    pub fn present(&mut self, sort_key: u64, image: Image<'a, R>, swapchain: Swapchain<'a, R>) {
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
