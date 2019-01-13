extern crate autograph_render;
#[macro_use]
extern crate autograph_render_macros;

use autograph_render::renderer::{BufferLayout, BufferTypeless, PrimitiveType, RendererBackend, TypeDesc};

#[derive(DescriptorSetInterface)]
struct TestDescriptorSetInterface<'a, R: RendererBackend> {
    #[descriptor(uniform_buffer, index = "0")]
    buffer: BufferTypeless<'a, R>,
}

#[test]
fn test_descriptor_set_interface() {}
