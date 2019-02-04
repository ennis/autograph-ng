extern crate autograph_render;
#[macro_use]
extern crate autograph_render_macros;

use autograph_render::{
    interface::PrimitiveType, interface::StructuredBufferData, interface::TypeDesc, Backend,
    BufferTypeless,
};

#[repr(C)]
#[derive(StructuredBufferData, Copy, Clone)]
struct TestLayout1 {
    a: i32,
    b: i32,
}

#[repr(C)]
#[derive(StructuredBufferData, Copy, Clone)]
struct TestLayout2 {
    a: [i32; 3],
    b: f32,
    c: [i32; 3],
}

#[test]
fn test_buffer_layout() {
    assert_eq!(
        <TestLayout1 as StructuredBufferData>::TYPE,
        &TypeDesc::Struct(&[
            (0, &TypeDesc::Primitive(PrimitiveType::Int)),
            (4, &TypeDesc::Primitive(PrimitiveType::Int)),
        ])
    );

    assert_eq!(
        <TestLayout2 as StructuredBufferData>::TYPE,
        &TypeDesc::Struct(&[
            (0, &TypeDesc::Vector(PrimitiveType::Int, 3)),
            (12, &TypeDesc::Primitive(PrimitiveType::Float)),
            (16, &TypeDesc::Vector(PrimitiveType::Int, 3)),
        ])
    );
}

#[test]
fn test_derive_in_function() {
    #[repr(C)]
    #[derive(StructuredBufferData, Copy, Clone)]
    struct TestLayout3 {
        a: [i32; 3],
        b: f32,
        c: [i32; 3],
    }

    assert_eq!(
        <TestLayout3 as StructuredBufferData>::TYPE,
        &TypeDesc::Struct(&[
            (0, &TypeDesc::Vector(PrimitiveType::Int, 3)),
            (12, &TypeDesc::Primitive(PrimitiveType::Float)),
            (16, &TypeDesc::Vector(PrimitiveType::Int, 3)),
        ])
    );
}
