#![feature(proc_macro_hygiene)]
use autograph_plugin::hot_reload_module;
use autograph_render::glm;
use autograph_render::framebuffer::Framebuffer;
use autograph_render::buffer::Buffer;
use autograph_render::pipeline::Viewport;
use autograph_render::descriptor::DescriptorSet;
use autograph_render::vertex::VertexData;
use autograph_render::buffer::StructuredBufferData;

#[derive(VertexData)]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Vertex2DTex {
    pub pos: [f32; 2],
    pub tex: [f32; 2],
}

impl Vertex2DTex {
    pub fn new(pos: [f32; 2], tex: [f32; 2]) -> Vertex2DTex {
        Vertex2DTex { pos, tex }
    }
}

#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
pub struct Uniforms {
    pub transform: glm::Mat4x3,
    pub resolution: glm::Vec2,
}

pub struct Blit<'a> {
    pub framebuffer: Framebuffer<'a>,
    pub per_object: DescriptorSet<'a>,
    pub viewport: Viewport,
    pub vertex_buffer: Buffer<'a, [Vertex2DTex]>,
}

#[hot_reload_module]
pub mod hot {
    use autograph_render::include_shader;

    #[no_mangle]
    pub static BLIT_VERT: &[u8] = include_shader!("blit.vert");
    #[no_mangle]
    pub static BLIT_FRAG: &[u8] = include_shader!("blit.frag");
}
