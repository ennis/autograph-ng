use gfx2::interface::{PrimitiveType, TypeDesc};
use std::cmp::max;

//--------------------------------------------------------------------------------------------------
// yet another copy of the align offset function
fn align_offset(ptr: usize, align: usize) -> usize {
    let offset = ptr % align;
    if offset == 0 {
        0
    } else {
        align - offset
    }
}

fn round_up(value: usize, multiple: usize) -> usize {
    if multiple == 0 {
        return value;
    }
    let remainder = value % multiple;
    if remainder == 0 {
        return value;
    }
    value + multiple - remainder
}

fn std140_array_align_and_size(elemty: &TypeDesc, size: usize) -> (usize, usize) {
    let (elem_align, elem_size) = std140_align_and_size(elemty);
    // alignment = column type align rounded up to vec4 align (16 bytes)
    let base_align = max(16, elem_align);
    let stride = elem_size + align_offset(elem_size, elem_align);
    // total array size = num columns * stride, rounded up to the next multiple of the base alignment
    let array_size = round_up(size as usize * stride, base_align);
    (base_align, array_size)
}

fn std140_align_and_size(ty: &TypeDesc) -> (usize, usize) {
    match *ty {
        TypeDesc::Primitive(PrimitiveType::Int)
        | TypeDesc::Primitive(PrimitiveType::UnsignedInt)
        | TypeDesc::Primitive(PrimitiveType::Float) => {
            //assert!(ty.width == 32);
            (4, 4)
        }
        TypeDesc::Vector(primty, num_components) => {
            let (_, n) = std140_align_and_size(&TypeDesc::Primitive(primty));
            match num_components {
                2 => (2 * n, 2 * n),
                3 => (4 * n, 3 * n),
                4 => (4 * n, 4 * n),
                _ => panic!("unsupported vector size"),
            }
        }
        TypeDesc::Matrix(primty, rows, cols) => {
            std140_array_align_and_size(&TypeDesc::Vector(primty, rows), cols as usize)
        }
        TypeDesc::Array(elemty, size) => match elemty {
            TypeDesc::Primitive(_) | TypeDesc::Vector(_, _) => {
                std140_array_align_and_size(elemty, size)
            }
            ty => panic!("unsupported array element type: {:?}", ty),
        },
        ty => panic!("unsupported type: {:?}", ty),
    }
}

//--------------------------------------------------------------------------------------------------
pub struct Std140LayoutBuilder {
    next_offset: usize,
}

impl Std140LayoutBuilder {
    pub fn new() -> Std140LayoutBuilder {
        Std140LayoutBuilder { next_offset: 0 }
    }

    fn align(&mut self, a: usize) -> usize {
        self.next_offset += align_offset(self.next_offset, a);
        self.next_offset
    }

    pub fn add_member(&mut self, ty: &TypeDesc) -> usize {
        let (align, size) = std140_align_and_size(ty);
        let current_offset = self.align(align);
        self.next_offset += size;
        current_offset
    }
}
