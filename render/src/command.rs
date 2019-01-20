use crate::buffer::BufferTypeless;
use crate::descriptor::DescriptorSet;
use crate::framebuffer::Framebuffer;
use crate::image::Image;
use crate::sync::PipelineStageFlags;
use crate::pipeline::GraphicsPipeline;
use crate::pipeline::PipelineInterface;
use crate::pipeline::PipelineInterfaceVisitor;
use crate::pipeline::ScissorRect;
use crate::pipeline::Viewport;
use crate::swapchain::Swapchain;
use crate::sync::MemoryBarrier;
use crate::vertex::IndexBufferDescriptor;
use crate::vertex::IndexFormat;
use crate::vertex::VertexBufferDescriptor;
use crate::traits;
use std::ops::Range;

/// Represents a command to be executed by the renderer backend.
///
/// Before being sent to the backend, all commands are collected into a single array, and then
/// sorted accorded to their `sortkey`. This sort is stable,
/// so if two commands in a command buffer have the same sortkey, the order of insertion is kept.
/// However, commands with the same sorting key from different command buffers
/// can end up interleaved.
#[derive(Clone)]
pub struct Command<'a> {
    pub sortkey: u64,
    pub cmd: CommandInner<'a>,
}

/// Parameters for non-indexed draw commands.
#[derive(Copy, Clone, Debug)]
pub struct DrawParams {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

/// Parameters for indexed draw commands.
#[derive(Copy, Clone, Debug)]
pub struct DrawIndexedParams {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

#[derive(Clone)]
pub enum CommandInner<'a> {
    // MAIN (LEAD-IN) COMMANDS ---------------------------------------------------------------------
    PipelineBarrier {},
    ClearImageFloat {
        image: &'a dyn traits::Image,
        color: [f32; 4],
    },
    ClearDepthStencilImage {
        image: &'a dyn traits::Image,
        depth: f32,
        stencil: Option<u8>,
    },
    Present {
        image: &'a dyn traits::Image,
        swapchain: &'a dyn traits::Swapchain,
    },
    DrawHeader {
        pipeline: &'a dyn traits::GraphicsPipeline,
    },

    // STATE CHANGE COMMANDS -----------------------------------------------------------------------
    SetDescriptorSets {
        descriptor_sets: Vec<&'a dyn traits::DescriptorSet>,
    },
    SetFramebuffer {
        framebuffer: &'a dyn traits::Framebuffer,
    },
    SetVertexBuffers {
        vertex_buffers: Vec<&'a dyn traits::Buffer>,
    },
    SetIndexBuffer {
        index_buffer: &'a dyn traits::Buffer,
        offset: usize,
        ty: IndexFormat,
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

/// Command buffers contain a list of commands.
pub struct CommandBuffer<'a> {
    commands: Vec<Command<'a>>,
}

/// API exposed by command buffers.
/// Can build multiple command buffers concurrently in different threads.
impl<'a> CommandBuffer<'a> {
    pub(super) fn new() -> CommandBuffer<'a> {
        CommandBuffer {
            commands: Vec::new(),
        }
    }

    fn push_command(&mut self, sortkey: u64, cmd: CommandInner<'a>) {
        self.commands.push(Command { cmd, sortkey })
    }

    // fn self.push_header_command(sortkey)
    // fn self.push_trailing_command()

    pub fn iter(&self) -> impl Iterator<Item = &Command<'a>> {
        self.commands.iter()
    }

    //----------------------------------------------------------------------------------------------
    // Manual sync

    /// Inserts an explicit pipeline barrier.
    pub fn pipeline_barrier(
        &mut self,
        _sort_key: u64,
        _src: PipelineStageFlags,
        _dst: PipelineStageFlags,
        _memory_barriers: &[MemoryBarrier],
    ) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Allocate

    //----------------------------------------------------------------------------------------------
    // Copy

    /// Copy data between buffers.
    pub fn copy_buffer(
        &mut self,
        _sort_key: u64,
        _src: BufferTypeless<'a>,
        _dst: BufferTypeless<'a>,
        _src_range: Range<u64>,
        _dst_range: Range<u64>,
    ) {
        unimplemented!()
    }

    //----------------------------------------------------------------------------------------------
    // Clear

    /// Clears an image.
    pub fn clear_image(&mut self, sortkey: u64, image: Image<'a>, color: &[f32; 4]) {
        self.push_command(
            sortkey,
            CommandInner::ClearImageFloat {
                image: image.0,
                color: *color,
            },
        )
    }

    /// Clears an image.
    pub fn clear_depth_stencil_image(
        &mut self,
        sortkey: u64,
        image: Image<'a>,
        depth: f32,
        stencil: Option<u8>,
    ) {
        self.push_command(
            sortkey,
            CommandInner::ClearDepthStencilImage {
                image: image.0,
                depth,
                stencil,
            },
        )
    }

    //----------------------------------------------------------------------------------------------
    // Draw

    fn set_descriptor_sets<I: IntoIterator<Item = DescriptorSet<'a>>>(
        &mut self,
        sortkey: u64,
        descriptor_sets: I,
    ) {
        self.push_command(
            sortkey,
            CommandInner::SetDescriptorSets {
                descriptor_sets: descriptor_sets.into_iter().map(|d| d.0).collect(),
            },
        )
    }

    fn set_framebuffer(&mut self, sortkey: u64, framebuffer: Framebuffer<'a>) {
        self.push_command(sortkey, CommandInner::SetFramebuffer { framebuffer: framebuffer.0 })
    }

    fn set_vertex_buffers<'tcx, I: IntoIterator<Item = VertexBufferDescriptor<'a, 'tcx>>>(
        &mut self,
        sortkey: u64,
        vertex_buffers: I,
    ) {
        self.push_command(
            sortkey,
            CommandInner::SetVertexBuffers {
                vertex_buffers: vertex_buffers.into_iter().map(|d| d.buffer.0).collect(),
            },
        )
    }

    fn set_index_buffer(
        &mut self,
        sortkey: u64,
        index_buffer: BufferTypeless<'a>,
        offset: usize,
        ty: IndexFormat,
    ) {
        self.push_command(
            sortkey,
            CommandInner::SetIndexBuffer {
                index_buffer: index_buffer.0,
                offset,
                ty,
            },
        )
    }

    fn set_viewports<I: IntoIterator<Item = Viewport>>(&mut self, sortkey: u64, viewports: I) {
        self.push_command(
            sortkey,
            CommandInner::SetViewports {
                viewports: viewports.into_iter().collect(),
            },
        )
    }

    fn set_scissors<I: IntoIterator<Item = ScissorRect>>(&mut self, sortkey: u64, scissors: I) {
        self.push_command(
            sortkey,
            CommandInner::SetScissors {
                scissors: scissors.into_iter().collect(),
            },
        )
    }

    fn bind_pipeline_interface<PI: PipelineInterface<'a>>(
        &mut self,
        sortkey: u64,
        pipeline: GraphicsPipeline<'a>,
        interface: &PI,
    ) {
        self.push_command(sortkey, CommandInner::DrawHeader { pipeline: pipeline.0 });

        struct Visitor<'a, 'b> {
            sortkey: u64,
            cmdbuf: &'b mut CommandBuffer<'a>,
        }

        impl<'a, 'b> PipelineInterfaceVisitor<'a> for Visitor<'a, 'b> {
            fn visit_descriptor_sets<I: IntoIterator<Item = DescriptorSet<'a>>>(
                &mut self,
                descriptor_sets: I,
            ) {
                self.cmdbuf
                    .set_descriptor_sets(self.sortkey, descriptor_sets);
            }

            fn visit_vertex_buffers<
                'tcx,
                I: IntoIterator<Item = VertexBufferDescriptor<'a, 'tcx>>,
            >(
                &mut self,
                vertex_buffers: I,
            ) {
                self.cmdbuf.set_vertex_buffers(self.sortkey, vertex_buffers);
            }

            fn visit_index_buffer(&mut self, buffer: IndexBufferDescriptor<'a>) {
                self.cmdbuf.set_index_buffer(
                    self.sortkey,
                    buffer.buffer,
                    buffer.offset as usize,
                    buffer.format,
                );
            }

            fn visit_framebuffer(&mut self, framebuffer: Framebuffer<'a>) {
                self.cmdbuf.set_framebuffer(self.sortkey, framebuffer);
            }

            fn visit_dynamic_viewports<I: IntoIterator<Item = Viewport>>(&mut self, viewports: I) {
                self.cmdbuf.set_viewports(self.sortkey, viewports);
            }

            fn visit_dynamic_scissors<I: IntoIterator<Item = ScissorRect>>(&mut self, scissors: I) {
                self.cmdbuf.set_scissors(self.sortkey, scissors);
            }
        }

        let mut v = Visitor {
            sortkey,
            cmdbuf: self,
        };

        interface.do_visit(&mut v);
    }

    pub fn draw<PI: PipelineInterface<'a>>(
        &mut self,
        sortkey: u64,
        pipeline: GraphicsPipeline<'a>,
        interface: &PI,
        params: DrawParams,
    ) {
        self.bind_pipeline_interface(sortkey, pipeline, interface);
        self.push_command(
            sortkey,
            CommandInner::Draw {
                vertex_count: params.vertex_count,
                instance_count: params.instance_count,
                first_vertex: params.first_vertex,
                first_instance: params.first_instance,
            },
        );
    }

    pub fn draw_indexed<PI: PipelineInterface<'a>>(
        &mut self,
        sortkey: u64,
        pipeline: GraphicsPipeline<'a>,
        interface: &PI,
        params: DrawIndexedParams,
    ) {
        self.bind_pipeline_interface(sortkey, pipeline, interface);
        self.push_command(
            sortkey,
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
    pub fn present(&mut self, sortkey: u64, image: Image<'a>, swapchain: Swapchain<'a>) {
        self.push_command(sortkey, CommandInner::Present { image: image.0, swapchain: swapchain.0 })
    }
}

/// TODO optimize (radix sort, dense command buffer layout, separate index map)
pub fn sort_command_buffers(cmdbufs: Vec<CommandBuffer>) -> Vec<Command> {
    let mut fused = Vec::new();
    //let mut sortkeys = Vec::new();
    //let mut i: usize = 0;
    for cmdbuf in cmdbufs.iter() {
        for cmd in cmdbuf.commands.iter() {
            fused.push(cmd.clone());
            //sortkeys.push(cmd.sortkey);
        }
    }

    fused.sort_by(|cmd_a, cmd_b| cmd_a.sortkey.cmp(&cmd_b.sortkey));
    fused
}
