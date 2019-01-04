#![feature(proc_macro_hygiene)]
pub mod blit;

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
