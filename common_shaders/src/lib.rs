#![feature(proc_macro_hygiene)]
use gfx2::glm;
use gfx2::*;
use gfx2_extension_runtime::hot_reload_module;

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

#[derive(BufferLayout, Copy, Clone)]
#[repr(C)]
pub struct Uniforms {
    pub transform: glm::Mat4x3,
    pub resolution: glm::Vec2,
}

pub struct Blit<'a, R: RendererBackend> {
    pub framebuffer: Framebuffer<'a, R>,
    pub per_object: DescriptorSet<'a, R>,
    pub viewport: Viewport,
    pub vertex_buffer: Buffer<'a, R, [Vertex2DTex]>,
}

#[hot_reload_module]
pub mod hot {
    use gfx2::include_shader;

    #[no_mangle]
    pub static BLIT_VERT: &[u8] = include_shader!("blit.vert");
    #[no_mangle]
    pub static BLIT_FRAG: &[u8] = include_shader!("blit.frag");
}
