pub trait Swapchain {
    fn size(&self) -> (u32, u32);
}

/*
pub trait ArgumentBlock {
    inherited: impl IntoIterator<Item = BareArgumentBlock<'a, B>>,
    descriptors: impl IntoIterator<Item = Descriptor<'a, B>>,
    vertex_buffers: impl IntoIterator<Item = VertexBufferView<'a, B>>,
    index_buffer: Option<IndexBufferView<'a, B>>,
    render_targets: impl IntoIterator<Item = RenderTargetView<'a, B>>,
    depth_stencil_target: Option<DepthStencilView<'a, B>>,
    viewports: impl IntoIterator<Item = Viewport>,
    scissors: impl IntoIterator<Item = Scissor>,
}*/
