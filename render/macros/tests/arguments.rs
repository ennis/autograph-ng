use autograph_render::{
    buffer::{Buffer, BufferTypeless},
    image::{TextureImageView},
    pipeline::{Arguments, Scissor, TypedArgumentBlock, Viewport},
    vertex::{IndexFormat, VertexData},
    Backend, DummyBackend,
};
use autograph_render::image::RenderTargetView;
use autograph_render::image::ImageView;

#[derive(VertexData, Copy, Clone)]
#[repr(C)]
struct Vertex {
    pos: [f32; 3],
    tex: [f32; 3],
}

#[derive(VertexData, Copy, Clone)]
#[repr(C)]
struct Vertex2 {
    color: [u8; 4],
    color2: [u8; 4],
}

#[derive(Arguments, Copy, Clone)]
struct TestInheritedArgs0<'a, B: Backend> {
    #[argument(render_target)]
    rt0: RenderTargetView<'a, B>,
    #[argument(render_target)]
    rt1: RenderTargetView<'a, B>,
    #[argument(depth_stencil_render_target)]
    dsrt: RenderTargetView<'a, B>,
    #[argument(viewport)]
    viewport: Viewport,
    #[argument(storage_image)]
    img0: ImageView<'a, B>,
    #[argument(storage_image)]
    img1: ImageView<'a, B>,
}

#[derive(Arguments, Copy, Clone)]
struct TestInheritedArgs1<'a, B: Backend> {
    #[argument(vertex_buffer)]
    vb0: Buffer<'a, B, [Vertex]>,
    #[argument(vertex_buffer)]
    vb1: Buffer<'a, B, [Vertex]>,
    #[argument(sampled_image)]
    tex0: TextureImageView<'a, B>,
    #[argument(sampled_image)]
    tex1: TextureImageView<'a, B>,
    #[argument(storage_buffer)]
    ssbo0: BufferTypeless<'a, B>,
}

#[derive(Arguments, Copy, Clone)]
struct TestInheritedArgs2<'a, B: Backend> {
    #[argument(inherit)]
    inherited: TypedArgumentBlock<'a, B, TestInheritedArgs0<'a, B>>,
    #[argument(render_target)]
    rt2: RenderTargetView<'a, B>,
    #[argument(render_target)]
    rt3: RenderTargetView<'a, B>,
    #[argument(scissor)]
    scissors: Scissor,
    #[argument(storage_image)]
    img2: ImageView<'a, B>,
}

#[derive(Arguments)]
struct TestArguments<'a, B: Backend> {
    #[argument(inherit)]
    inherited1: TypedArgumentBlock<'a, B, TestInheritedArgs1<'a, B>>,
    #[argument(inherit)]
    inherited2: TypedArgumentBlock<'a, B, TestInheritedArgs2<'a, B>>,
    #[argument(render_target)]
    rt4: RenderTargetView<'a, B>,
    #[argument(render_target)]
    rt5: RenderTargetView<'a, B>,
    #[argument(uniform_buffer)]
    ubo1: BufferTypeless<'a, B>,
    #[argument(uniform_buffer)]
    ubo2: BufferTypeless<'a, B>,
    #[argument(storage_buffer)]
    ssbo1: BufferTypeless<'a, B>,
    #[argument(sampled_image)]
    tex2: TextureImageView<'a, B>,
    #[argument(storage_image)]
    img3: ImageView<'a, B>,
    #[argument(viewport)]
    viewport: Viewport,
    #[argument(scissor)]
    scissors: Scissor,
    #[argument(vertex_buffer)]
    vb2: Buffer<'a, B, [Vertex2]>,
    #[argument(vertex_buffer)]
    vb3: Buffer<'a, B, [Vertex2]>,
    #[argument(index_buffer)]
    indices: Buffer<'a, B, [u16]>,
}

const ARGS_0_SIGNATURE: &'static autograph_render::pipeline::SignatureDescription<'static> =
    &autograph_render::pipeline::SignatureDescription {
        is_root_fragment_output_signature: true,
        is_root_vertex_input_signature: false,
        inherited: &[],
        descriptors: &[
            autograph_render::descriptor::DescriptorBinding {
                binding: 0usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::StorageImage,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
            autograph_render::descriptor::DescriptorBinding {
                binding: 1usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::StorageImage,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
        ],
        vertex_layouts: &[],
        fragment_outputs: &[
            autograph_render::framebuffer::FragmentOutputDescription {},
            autograph_render::framebuffer::FragmentOutputDescription {},
        ],
        depth_stencil_fragment_output: Some(
            autograph_render::framebuffer::FragmentOutputDescription {},
        ),
        index_format: None,
        num_viewports: 1usize,
        num_scissors: 0usize,
    };

const ARGS_1_SIGNATURE: &'static autograph_render::pipeline::SignatureDescription<'static> =
    &autograph_render::pipeline::SignatureDescription {
        is_root_fragment_output_signature: false,
        is_root_vertex_input_signature: false,
        inherited: &[],
        descriptors: &[
            autograph_render::descriptor::DescriptorBinding {
                binding: 0usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::SampledImage,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
            autograph_render::descriptor::DescriptorBinding {
                binding: 1usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::SampledImage,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
            autograph_render::descriptor::DescriptorBinding {
                binding: 2usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::StorageBuffer,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
        ],
        vertex_layouts: &[Vertex::LAYOUT, Vertex::LAYOUT],
        fragment_outputs: &[],
        depth_stencil_fragment_output: None,
        index_format: None,
        num_viewports: 0usize,
        num_scissors: 0usize,
    };

const ARGS_2_SIGNATURE: &'static autograph_render::pipeline::SignatureDescription<'static> =
    &autograph_render::pipeline::SignatureDescription {
        is_root_fragment_output_signature: true,
        is_root_vertex_input_signature: false,
        inherited: &[ARGS_0_SIGNATURE],
        descriptors: &[autograph_render::descriptor::DescriptorBinding {
            binding: 0usize,
            descriptor_type: autograph_render::descriptor::DescriptorType::StorageImage,
            stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
            tydesc: None,
        }],
        vertex_layouts: &[],
        fragment_outputs: &[
            autograph_render::framebuffer::FragmentOutputDescription {},
            autograph_render::framebuffer::FragmentOutputDescription {},
        ],
        depth_stencil_fragment_output: None,
        index_format: None,
        num_viewports: 0usize,
        num_scissors: 1usize,
    };

const SIGNATURE: &'static autograph_render::pipeline::SignatureDescription<'static> =
    &autograph_render::pipeline::SignatureDescription {
        is_root_fragment_output_signature: true,
        is_root_vertex_input_signature: false,
        inherited: &[ARGS_1_SIGNATURE, ARGS_2_SIGNATURE],
        descriptors: &[
            autograph_render::descriptor::DescriptorBinding {
                binding: 0usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::UniformBuffer,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
            autograph_render::descriptor::DescriptorBinding {
                binding: 1usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::UniformBuffer,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
            autograph_render::descriptor::DescriptorBinding {
                binding: 2usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::StorageBuffer,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
            autograph_render::descriptor::DescriptorBinding {
                binding: 3usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::SampledImage,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
            autograph_render::descriptor::DescriptorBinding {
                binding: 4usize,
                descriptor_type: autograph_render::descriptor::DescriptorType::StorageImage,
                stage_flags: autograph_render::pipeline::ShaderStageFlags::ALL_GRAPHICS,
                count: 1,
                tydesc: None,
            },
        ],
        vertex_layouts: &[Vertex2::LAYOUT, Vertex2::LAYOUT],
        fragment_outputs: &[
            autograph_render::framebuffer::FragmentOutputDescription {},
            autograph_render::framebuffer::FragmentOutputDescription {},
        ],
        depth_stencil_fragment_output: None,
        index_format: Some(IndexFormat::U16),
        num_viewports: 1usize,
        num_scissors: 1usize,
    };

#[test]
fn arguments_check() {
    assert_eq!(
        <TestInheritedArgs0<DummyBackend> as Arguments<DummyBackend>>::SIGNATURE,
        ARGS_0_SIGNATURE
    );
    assert_eq!(
        <TestInheritedArgs1<DummyBackend> as Arguments<DummyBackend>>::SIGNATURE,
        ARGS_1_SIGNATURE
    );
    assert_eq!(
        <TestInheritedArgs2<DummyBackend> as Arguments<DummyBackend>>::SIGNATURE,
        ARGS_2_SIGNATURE
    );
    assert_eq!(
        <TestArguments<DummyBackend> as Arguments<DummyBackend>>::SIGNATURE,
        SIGNATURE
    );
}
