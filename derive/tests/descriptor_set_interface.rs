extern crate gfx2;
#[macro_use]
extern crate gfx2_derive;

use gfx2::renderer::backend::gl::OpenGlBackend;
use gfx2::renderer::{BufferLayout, BufferTypeless, PrimitiveType, RendererBackend, TypeDesc};

#[derive(DescriptorSetInterface)]
struct TestDescriptorSetInterface<'a, R: RendererBackend> {
    #[descriptor(uniform_buffer, index = "0")]
    buffer: BufferTypeless<'a, R>,
}

#[test]
fn test_descriptor_set_interface() {}
