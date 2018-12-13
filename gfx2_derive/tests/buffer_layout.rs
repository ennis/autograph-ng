extern crate gfx2;
#[macro_use]
extern crate gfx2_derive;

use gfx2::renderer::backend::gl::OpenGlBackend;
use gfx2::renderer::{BufferLayout, BufferTypeless, PrimitiveType, RendererBackend, TypeDesc};

#[repr(C)]
#[derive(BufferLayout, Copy, Clone)]
struct TestLayout1 {
    a: i32,
    b: i32,
}

#[repr(C)]
#[derive(BufferLayout, Copy, Clone)]
struct TestLayout2 {
    a: [i32; 3],
    b: f32,
    c: [i32; 3],
}

#[test]
fn test_buffer_layout() {
    assert_eq!(
        <TestLayout1 as gfx2::renderer::BufferLayout>::TYPE,
        &TypeDesc::Struct(&[
            (0, &TypeDesc::Primitive(PrimitiveType::Int)),
            (4, &TypeDesc::Primitive(PrimitiveType::Int)),
        ])
    );

    assert_eq!(
        <TestLayout2 as gfx2::renderer::BufferLayout>::TYPE,
        &TypeDesc::Struct(&[
            (0, &TypeDesc::Vector(PrimitiveType::Int, 3)),
            (12, &TypeDesc::Primitive(PrimitiveType::Float)),
            (16, &TypeDesc::Vector(PrimitiveType::Int, 3)),
        ])
    );
}
