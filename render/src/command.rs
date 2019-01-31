use crate::buffer::BufferTypeless;
use crate::image::Image;
use crate::pipeline::GraphicsPipeline;
use crate::pipeline::PipelineInterface;
use crate::swapchain::Swapchain;
use crate::sync::MemoryBarrier;
use crate::sync::PipelineStageFlags;
use crate::traits;
use crate::Arena;
use std::ops::Range;
use crate::pipeline::PipelineArguments;

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
    SetPipelineArguments {
        arguments: &'a dyn traits::PipelineArguments,
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
    fn set_pipeline<P: PipelineInterface<'a>>(&mut self, sortkey: u64, pipeline: GraphicsPipeline<'a, P>, arguments: PipelineArguments<'a, P>)
    {
        self.push_command(sortkey, CommandInner::DrawHeader {
            pipeline: pipeline.0
        });
        self.push_command(
            sortkey,
            CommandInner::SetPipelineArguments {
                arguments: arguments.0
            },
        )
    }

    pub fn draw<P: PipelineInterface<'a>>(
        &mut self,
        sortkey: u64,
        _arena: &'a Arena,
        pipeline: GraphicsPipeline<'a, P>,
        arguments: PipelineArguments<'a, P>,
        params: DrawParams,
    ) {

        self.set_pipeline(sortkey, pipeline, arguments);
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

    pub fn draw_indexed<P: PipelineInterface<'a>>(
        &mut self,
        sortkey: u64,
        _arena: &'a Arena,
        pipeline: GraphicsPipeline<'a, P>,
        arguments: PipelineArguments<'a, P>,
        params: DrawIndexedParams,
    ) {
        self.set_pipeline(sortkey, pipeline, arguments);
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
        self.push_command(
            sortkey,
            CommandInner::Present {
                image: image.0,
                swapchain: swapchain.0,
            },
        )
    }
}

/// TODO optimize (radix sort, dense command buffer layout, separate index map)
pub fn sort_command_buffers<'a>(cmdbufs: Vec<CommandBuffer<'a>>) -> Vec<Command<'a>> {
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
