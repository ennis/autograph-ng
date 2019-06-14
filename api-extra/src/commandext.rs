use crate::quad::Quad;
use autograph_api::{
    command::{CommandBuffer, DrawParams},
    pipeline::{Arguments, GraphicsPipeline, TypedGraphicsPipeline, TypedSignature},
    Arena, Backend,
};

pub trait CommandBufferExt<'a, B: Backend> {
    fn draw_quad<A: Arguments<'a, B>>(
        &mut self,
        sortkey: u64,
        arena: &'a Arena<B>,
        pipeline: TypedGraphicsPipeline<'a, B, Quad<'a, B, A>>,
        arguments: A,
    );
}

impl<'a, B: Backend> CommandBufferExt<'a, B> for CommandBuffer<'a, B> {
    fn draw_quad<A: Arguments<'a, B>>(
        &mut self,
        sortkey: u64,
        arena: &'a Arena<B>,
        pipeline: GraphicsPipeline<'a, B, TypedSignature<'a, B, Quad<'a, B, A>>>,
        arguments: A,
    ) {
        self.draw(
            sortkey,
            arena,
            pipeline,
            Quad::new(arguments),
            DrawParams::quad(),
        )
    }
}
