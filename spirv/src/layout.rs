use crate::{PrimitiveType, TypeDesc};
use dropless_arena::DroplessArena;
use std::iter;

//--------------------------------------------------------------------------------------------------
// yet another copy of the align offset function
/*fn align_offset(ptr: usize, align: usize) -> usize {
    let offset = ptr % align;
    if offset == 0 {
        0
    } else {
        align - offset
    }
}*/

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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Layout<'tcx> {
    pub align: usize,
    pub size: usize,
    pub details: LayoutDetails<'tcx>,
}

impl<'tcx> Layout<'tcx> {
    pub const fn with_size_align(size: usize, align: usize) -> Layout<'tcx> {
        Layout {
            align,
            size,
            details: LayoutDetails::None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct FieldsLayout<'tcx> {
    pub offsets: &'tcx [usize],
    pub layouts: &'tcx [&'tcx Layout<'tcx>],
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArrayLayout<'tcx> {
    pub elem_layout: &'tcx Layout<'tcx>,
    pub stride: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum LayoutDetails<'tcx> {
    None,
    Array(ArrayLayout<'tcx>),
    Struct(FieldsLayout<'tcx>),
}

fn std140_array_layout<'tcx>(
    a: &'tcx DroplessArena,
    elem_ty: &TypeDesc,
    arraylen: usize,
) -> &'tcx Layout<'tcx> {
    let elem_layout = std140_layout(a, elem_ty);
    // alignment = column type align rounded up to vec4 align (16 bytes)
    let base_align = round_up(elem_layout.align, 16);
    let stride = round_up(elem_layout.size, elem_layout.align);
    // total array size = num columns * stride, rounded up to the next multiple of the base alignment.
    // actually the spec says nothing about the 'size' of an element, only about the alignment
    // of the next element in the structure.
    let array_size = round_up(arraylen * stride, base_align);
    a.alloc(Layout {
        align: base_align,
        size: array_size,
        details: LayoutDetails::Array(ArrayLayout {
            stride,
            elem_layout,
        }),
    })
}

fn std140_struct_layout<'tcx>(a: &'tcx DroplessArena, fields: &[&TypeDesc]) -> &'tcx Layout<'tcx> {
    /* If the member is a structure, the base alignment of the structure is N,
    where N is the largest base alignment value of any of its members,
    and rounded up to the base alignment of a vec4.
    The individual members of this sub-structure are then assigned offsets by applying this set of rules recursively,
    where the base offset of the first member of the sub-structure is equal to the aligned offset of the structure.
    The structure may have padding at the end;
    the base offset of the member following the sub-structure is rounded up to the next multiple of the base alignment of the structure.
    */
    // TODO: zero-sized structures?

    let layouts : Vec<_> = fields.iter().map(|&mty| std140_layout(a, mty)).collect();
    let layouts = a.alloc_extend(layouts.into_iter());
    let n = layouts.iter().map(|l| l.align).max().unwrap_or(0);
    if n == 0 {
        // skip, no members
        return a.alloc(Layout {
            align: 0,
            size: 0,
            details: LayoutDetails::Struct(FieldsLayout {
                offsets: &[],
                layouts: &[],
            }),
        });
    }

    // round up to base alignment of vec4
    let n = round_up(n, 16);

    // compute field offsets
    let offsets = a.alloc_extend(iter::repeat(0).take(fields.len()));
    let mut off = 0;
    for i in 0..fields.len() {
        offsets[i] = off;
        off += layouts[i].size;
    }

    // round up total size to base align
    let size = round_up(off, n);

    a.alloc(Layout {
        align: n,
        size,
        details: LayoutDetails::Struct(FieldsLayout { layouts, offsets }),
    })
}

fn std140_primitive_layout(prim_ty: PrimitiveType) -> Layout<'static> {
    match prim_ty {
        PrimitiveType::Int | PrimitiveType::UnsignedInt | PrimitiveType::Float => Layout {
            size: 4,
            align: 4,
            details: LayoutDetails::None,
        },
        _ => unimplemented!(),
    }
}

fn std140_vector_layout(prim_ty: PrimitiveType, len: u8) -> Layout<'static> {
    let Layout { size: n, .. } = std140_primitive_layout(prim_ty);
    match len {
        2 => Layout {
            align: 2 * n,
            size: 2 * n,
            details: LayoutDetails::None,
        },
        3 => Layout {
            align: 4 * n,
            size: 3 * n,
            details: LayoutDetails::None,
        },
        4 => Layout {
            align: 4 * n,
            size: 4 * n,
            details: LayoutDetails::None,
        },
        _ => panic!("unsupported vector size"),
    }
}

fn std140_layout<'tcx>(a: &'tcx DroplessArena, ty: &TypeDesc) -> &'tcx Layout<'tcx> {
    match *ty {
        TypeDesc::Primitive(p) => a.alloc(std140_primitive_layout(p)),
        TypeDesc::Vector { elem_ty, len } => a.alloc(std140_vector_layout(elem_ty, len)),
        TypeDesc::Matrix {
            elem_ty,
            rows,
            columns,
        } => std140_array_layout(
            a,
            &TypeDesc::Vector { elem_ty, len: rows },
            columns as usize,
        ),
        TypeDesc::Array { elem_ty, len } => match elem_ty {
            TypeDesc::Primitive(_) | TypeDesc::Vector { .. } | TypeDesc::Struct { .. } => {
                std140_array_layout(a, elem_ty, len)
            }
            ty => panic!("unsupported array element type: {:?}", ty),
        },
        TypeDesc::Struct { fields } => std140_struct_layout(a, fields),
        ty => panic!("unsupported type: {:?}", ty),
    }
}

impl<'tcx> Layout<'tcx> {
    pub fn std140(a: &'tcx DroplessArena, ty: &TypeDesc) -> &'tcx Layout<'tcx> {
        std140_layout(a, ty)
    }
}
