use crate::PrimitiveType;
use crate::TypeDesc;

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

/// TODO more unit testing!
pub struct Std140AlignAndSize {
    pub align: usize,
    /// More precisely, "offset to next member"
    pub size: usize,
}

impl Std140AlignAndSize {
    pub fn of_array(elemty: &TypeDesc, arraylen: usize) -> Std140AlignAndSize {
        let Std140AlignAndSize {
            align: elem_align,
            size: elem_size,
        } = Std140AlignAndSize::of(elemty);
        // alignment = column type align rounded up to vec4 align (16 bytes)
        let base_align = round_up(elem_align, 16);
        let stride = elem_size + align_offset(elem_size, elem_align);
        // total array size = num columns * stride, rounded up to the next multiple of the base alignment.
        // actually the spec says nothing about the 'size' of an element, only about the alignment
        // of the next element in the structure.
        let array_size = round_up(arraylen * stride, base_align);
        Std140AlignAndSize {
            align: base_align,
            size: array_size,
        }
    }

    /// returns true if round-up needed after (for items following structures)
    pub fn of(ty: &TypeDesc) -> Std140AlignAndSize {
        match *ty {
            TypeDesc::Primitive(PrimitiveType::Int)
            | TypeDesc::Primitive(PrimitiveType::UnsignedInt)
            | TypeDesc::Primitive(PrimitiveType::Float) => {
                //assert!(ty.width == 32);
                Std140AlignAndSize { align: 4, size: 4 }
            }
            TypeDesc::Vector(primty, num_components) => {
                let Std140AlignAndSize { size: n, .. } =
                    Std140AlignAndSize::of(&TypeDesc::Primitive(primty));
                match num_components {
                    2 => Std140AlignAndSize {
                        align: 2 * n,
                        size: 2 * n,
                    },
                    3 => Std140AlignAndSize {
                        align: 4 * n,
                        size: 3 * n,
                    },
                    4 => Std140AlignAndSize {
                        align: 4 * n,
                        size: 4 * n,
                    },
                    _ => panic!("unsupported vector size"),
                }
            }
            TypeDesc::Matrix(primty, rows, cols) => {
                Std140AlignAndSize::of_array(&TypeDesc::Vector(primty, rows), cols as usize)
            }
            TypeDesc::Array(elemty, size, _) => match elemty {
                TypeDesc::Primitive(_) | TypeDesc::Vector(_, _) | TypeDesc::Struct(_) => {
                    Std140AlignAndSize::of_array(elemty, size)
                }
                ty => panic!("unsupported array element type: {:?}", ty),
            },
            TypeDesc::Struct(layout) => {
                /* If the member is a structure, the base alignment of the structure is N,
                where N is the largest base alignment value of any of its members,
                and rounded up to the base alignment of a vec4.
                The individual members of this sub-structure are then assigned offsets by applying this set of rules recursively,
                where the base offset of the first member of the sub-structure is equal to the aligned offset of the structure.
                The structure may have padding at the end;
                the base offset of the member following the sub-structure is rounded up to the next multiple of the base alignment of the structure.
                */
                // TODO: zero-sized structures?
                let n = layout
                    .fields
                    .iter()
                    .map(|(_, mty)| Std140AlignAndSize::of(mty).align)
                    .max()
                    .unwrap_or(0);
                if n == 0 {
                    // skip, no members
                    return Std140AlignAndSize { align: 0, size: 0 };
                }

                // round up to base alignment of vec4
                let n = round_up(n, 16);

                // compute total structure size
                let mlast = layout.fields.last().unwrap();
                let mlast_offset = mlast.0;
                let mlast_size = Std140AlignAndSize::of(mlast.1).size;
                let size = mlast_offset + mlast_size;

                // round up total size to base align
                let size = round_up(size, n);

                Std140AlignAndSize { align: n, size }
            }
            ty => panic!("unsupported type: {:?}", ty),
        }
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
        let Std140AlignAndSize { align, size } = Std140AlignAndSize::of(ty);
        let current_offset = self.align(align);
        self.next_offset += size;
        current_offset
    }
}
