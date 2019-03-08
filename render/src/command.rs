use crate::{
    image::{DepthStencilView, Image2dView, RenderTargetView},
    pipeline::{Arguments, GraphicsPipeline, TypedSignature},
    swapchain::Swapchain,
    Arena, Backend,
};

/// Represents a command to be executed by the renderer backend.
///
/// Before being sent to the backend, all commands are collected into a single array, and then
/// sorted accorded to their `sortkey`. This sort is stable,
/// so if two commands in a command buffer have the same sortkey, the order of insertion is kept.
/// However, commands with the same sorting key from different command buffers
/// can end up interleaved.
#[derive(Clone)]
pub struct Command<'a, B: Backend> {
    pub sortkey: u64,
    pub cmd: CommandInner<'a, B>,
}

/// Parameters for non-indexed draw commands.
#[derive(Copy, Clone, Debug)]
pub struct DrawParams {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

impl DrawParams {
    /// Draw parameters for a quad (1 instance, 6 verts)
    pub fn quad() -> DrawParams {
        DrawParams {
            instance_count: 1,
            first_instance: 0,
            vertex_count: 6,
            first_vertex: 0,
        }
    }
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

#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""))]
pub enum CommandInner<'a, B: Backend> {
    // MAIN (LEAD-IN) COMMANDS ---------------------------------------------------------------------
    PipelineBarrier {},
    ClearImageFloat {
        image: &'a B::Image,
        color: [f32; 4],
    },
    ClearDepthStencilImage {
        image: &'a B::Image,
        depth: f32,
        stencil: Option<u8>,
    },
    Present {
        image: &'a B::Image,
        swapchain: &'a B::Swapchain,
    },
    DrawHeader {
        pipeline: &'a B::GraphicsPipeline,
    },

    // STATE CHANGE COMMANDS -----------------------------------------------------------------------
    SetPipelineArguments {
        arguments: &'a B::ArgumentBlock,
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
pub struct CommandBuffer<'a, B: Backend> {
    commands: Vec<Command<'a, B>>,
}

/// API exposed by command buffers.
/// Can build multiple command buffers concurrently in different threads.
impl<'a, B: Backend> CommandBuffer<'a, B> {
    pub(super) fn new() -> CommandBuffer<'a, B> {
        CommandBuffer {
            commands: Vec::new(),
        }
    }

    fn push_command(&mut self, sortkey: u64, cmd: CommandInner<'a, B>) {
        self.commands.push(Command { cmd, sortkey })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Command<'a, B>> {
        self.commands.iter()
    }

    //----------------------------------------------------------------------------------------------
    // Copy

    /*/// Copy data between buffers.
    pub fn copy_buffer(
        &mut self,
        _sort_key: u64,
        _src: BufferTypeless<'a, B>,
        _dst: BufferTypeless<'a, B>,
        _src_range: Range<u64>,
        _dst_range: Range<u64>,
    ) {
        unimplemented!()
    }*/

    //----------------------------------------------------------------------------------------------
    // Clear

    /// Clears an image.
    ///
    /// Q: Should it be necessary for the image to be an RTV?
    pub fn clear_render_target(
        &mut self,
        sortkey: u64,
        image: impl Into<RenderTargetView<'a, B>>,
        color: &[f32; 4],
    ) {
        self.push_command(
            sortkey,
            CommandInner::ClearImageFloat {
                image: image.into().image,
                color: *color,
            },
        )
    }

    /// Clears an image.
    pub fn clear_depth_stencil(
        &mut self,
        sortkey: u64,
        image: impl Into<DepthStencilView<'a, B>>,
        depth: f32,
        stencil: Option<u8>,
    ) {
        self.push_command(
            sortkey,
            CommandInner::ClearDepthStencilImage {
                image: image.into().image,
                depth,
                stencil,
            },
        )
    }

    //----------------------------------------------------------------------------------------------
    // Draw
    fn set_pipeline(
        &mut self,
        sortkey: u64,
        pipeline: &'a B::GraphicsPipeline,
        arguments: &'a B::ArgumentBlock,
    ) {
        self.push_command(sortkey, CommandInner::DrawHeader { pipeline });
        self.push_command(sortkey, CommandInner::SetPipelineArguments { arguments })
    }

    pub fn draw<P: Arguments<'a, B>>(
        &mut self,
        sortkey: u64,
        arena: &'a Arena<B>,
        pipeline: GraphicsPipeline<'a, B, TypedSignature<'a, B, P::IntoInterface>>,
        arguments: P,
        params: DrawParams,
    ) {
        let arguments = arguments.into_block(pipeline.signature, arena);
        self.set_pipeline(sortkey, pipeline.inner, arguments.arguments);
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

    pub fn draw_indexed<P: Arguments<'a, B>>(
        &mut self,
        sortkey: u64,
        arena: &'a Arena<B>,
        pipeline: GraphicsPipeline<'a, B, TypedSignature<'a, B, P::IntoInterface>>,
        arguments: P,
        params: DrawIndexedParams,
    ) {
        let arguments = arguments.into_block(pipeline.signature, arena);
        self.set_pipeline(sortkey, pipeline.inner, arguments.arguments);
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
    ///
    /// Q: What type should `image` be? Need a 2D view of an image which supports transfer source.
    pub fn present(
        &mut self,
        sortkey: u64,
        image: impl Into<Image2dView<'a, B>>,
        swapchain: Swapchain<'a, B>,
    ) {
        self.push_command(
            sortkey,
            CommandInner::Present {
                image: image.into().image,
                swapchain: swapchain.0,
            },
        )
    }
}

/// TODO optimize (radix sort, dense command buffer layout, separate index map)
pub fn sort_command_buffers<'a, B: Backend>(
    cmdbufs: impl IntoIterator<Item = CommandBuffer<'a, B>>,
) -> Vec<Command<'a, B>> {
    let mut fused = Vec::new();
    //let mut sortkeys = Vec::new();
    //let mut i: usize = 0;
    for cmdbuf in cmdbufs.into_iter() {
        for cmd in cmdbuf.commands.iter() {
            fused.push(cmd.clone());
            //sortkeys.push(cmd.sortkey);
        }
    }

    fused.sort_by(|cmd_a, cmd_b| cmd_a.sortkey.cmp(&cmd_b.sortkey));
    fused
}
