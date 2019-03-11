//use super::parse::SpirvModule;
use crate::{inst::*, IPtr, ImageType, Module, PrimitiveType, TypeDesc};
use dropless_arena::DroplessArena;
use spirv_headers::*;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ParsedDecoration {
    Block,
    BufferBlock,
    Constant,
    Location(u32),
    Index(u32),
    Binding(u32),
    DescriptorSet(u32),
    Uniform,
    Other(Decoration),
}

#[derive(Copy, Clone, Debug)]
pub struct Variable<'tcx> {
    pub id: u32,
    pub ty: &'tcx TypeDesc<'tcx>,
    pub deco: &'tcx [(IPtr, ParsedDecoration)],
    pub storage: StorageClass,
}

impl<'tcx> Variable<'tcx> {
    pub fn decorations(&self) -> impl Iterator<Item = &(IPtr, ParsedDecoration)> {
        self.deco.iter()
    }

    pub fn location_decoration(&self) -> Option<(IPtr, u32)> {
        self.deco
            .iter()
            .filter_map(|(iptr, d)| match d {
                ParsedDecoration::Location(loc) => Some((*iptr, *loc)),
                _ => None,
            })
            .next()
    }

    pub fn has_block_decoration(&self) -> Option<IPtr> {
        self.deco
            .iter()
            .find(|(_, d)| match d {
                ParsedDecoration::Block => true,
                _ => false,
            })
            .map(|d| d.0)
    }

    pub fn has_buffer_block_decoration(&self) -> Option<IPtr> {
        self.deco
            .iter()
            .find(|(_, d)| match d {
                ParsedDecoration::BufferBlock => true,
                _ => false,
            })
            .map(|d| d.0)
    }

    pub fn descriptor_set_decoration(&self) -> Option<(IPtr, u32)> {
        self.deco
            .iter()
            .filter_map(|(iptr, d)| match d {
                ParsedDecoration::DescriptorSet(ds) => Some((*iptr, *ds)),
                _ => None,
            })
            .next()
    }

    pub fn binding_decoration(&self) -> Option<(IPtr, u32)> {
        self.deco
            .iter()
            .filter_map(|(iptr, d)| match d {
                ParsedDecoration::Binding(ds) => Some((*iptr, *ds)),
                _ => None,
            })
            .next()
    }
}

pub struct Ast<'tcx> {
    _tymap: HashMap<u32, &'tcx TypeDesc<'tcx>>,
    vars: &'tcx [(IPtr, Variable<'tcx>)],
}

impl<'tcx> Ast<'tcx> {
    pub fn new(arena: &'tcx DroplessArena, module: &Module) -> Ast<'tcx> {
        let tymap = parse_types(arena, module);
        let vars = parse_variables(arena, module, &tymap);
        Ast {
            _tymap: tymap,
            vars,
        }
    }

    pub fn variables(&self) -> impl Iterator<Item = &'tcx (IPtr, Variable<'tcx>)> {
        self.vars.iter()
    }

    //pub fn uniform_buffers(&self)
}

fn parse_types<'tcx>(a: &'tcx DroplessArena, m: &Module) -> HashMap<u32, &'tcx TypeDesc<'tcx>> {
    // build a map from id to type
    let mut tymap = HashMap::<u32, &'tcx TypeDesc<'tcx>>::new();

    // can process types in order, since the spec specifies that:
    // "Types are built bottom up: A parameterizing operand in a type must be defined before being used."
    m.decode().for_each(|(_, inst)| {
        match &inst {
            Instruction::TypeVoid(ITypeVoid { result_id }) => {
                tymap.insert(*result_id, a.alloc(TypeDesc::Void));
            }
            Instruction::TypeBool(ITypeBool { result_id }) => {
                tymap.insert(
                    *result_id,
                    a.alloc(TypeDesc::Primitive(PrimitiveType::Bool)),
                );
            }
            Instruction::TypeInt(ITypeInt {
                result_id,
                width,
                signedness,
            }) => {
                assert_eq!(*width, 32, "unsupported bit width");
                match signedness {
                    true => {
                        tymap.insert(*result_id, a.alloc(TypeDesc::Primitive(PrimitiveType::Int)))
                    }
                    false => tymap.insert(
                        *result_id,
                        a.alloc(TypeDesc::Primitive(PrimitiveType::UnsignedInt)),
                    ),
                };
            }
            Instruction::TypeFloat(ITypeFloat { result_id, width }) => {
                assert_eq!(*width, 32, "unsupported bit width");
                tymap.insert(
                    *result_id,
                    a.alloc(TypeDesc::Primitive(PrimitiveType::Float)),
                );
            }
            Instruction::TypeVector(ITypeVector {
                result_id,
                component_id,
                count,
            }) => {
                let elem_ty = tymap[component_id];
                if let &TypeDesc::Primitive(elem_ty) = &*elem_ty {
                    tymap.insert(
                        *result_id,
                        a.alloc(TypeDesc::Vector {
                            elem_ty,
                            len: *count as u8,
                        }),
                    );
                } else {
                    panic!("expected primitive type");
                }
            }
            Instruction::TypeMatrix(ITypeMatrix {
                result_id,
                column_type_id,
                column_count,
            }) => {
                let colty = tymap[column_type_id];
                if let &TypeDesc::Vector { elem_ty, len } = colty {
                    tymap.insert(
                        *result_id,
                        a.alloc(TypeDesc::Matrix {
                            elem_ty,
                            rows: len,
                            columns: *column_count as u8,
                        }),
                    );
                } else {
                    panic!("expected vector type");
                }
            }
            Instruction::TypeImage(ITypeImage {
                result_id,
                sampled_type_id,
                dim,
                depth: _,
                arrayed: _,
                ms: _,
                sampled: _,
                format,
                access: _,
            }) => {
                // TODO yeah that's not really enough
                let sampled_ty = tymap[sampled_type_id];
                tymap.insert(
                    *result_id,
                    a.alloc(TypeDesc::Image(ImageType {
                        sampled_ty,
                        format: *format,
                        dimensions: *dim,
                    })),
                );
            }
            Instruction::TypeSampler(ITypeSampler { result_id: _ }) => unimplemented!(),
            Instruction::TypeSampledImage(ITypeSampledImage {
                result_id,
                image_type_id,
            }) => {
                let image_ty = tymap[image_type_id];
                if let &TypeDesc::Image(ref img_ty) = image_ty {
                    tymap.insert(*result_id, a.alloc(TypeDesc::SampledImage(img_ty)));
                } else {
                    panic!("expected image type")
                };
            }
            Instruction::TypeArray(ITypeArray {
                result_id,
                type_id,
                length_id: _,
            }) => {
                let elem_ty = tymap[type_id];
                tymap.insert(*result_id, a.alloc(TypeDesc::Array { elem_ty, len: 0 }));
            }
            Instruction::TypeRuntimeArray(ITypeRuntimeArray { result_id, type_id }) => {
                let elem_ty = tymap[type_id];
                tymap.insert(*result_id, a.alloc(TypeDesc::Array { elem_ty, len: 0 }));
            }
            Instruction::TypeStruct(ITypeStruct {
                result_id,
                member_types,
            }) => {
                let fields = a.alloc_extend(member_types.iter().map(|tyid| tymap[tyid]));
                tymap.insert(*result_id, a.alloc(TypeDesc::Struct { fields }));
            }
            Instruction::TypeOpaque(ITypeOpaque {
                result_id: _,
                name: _,
            }) => unimplemented!(),
            Instruction::TypePointer(ITypePointer {
                result_id,
                storage_class: _,
                type_id,
            }) => {
                let ty = tymap[type_id];
                tymap.insert(*result_id, a.alloc(TypeDesc::Pointer(ty)));
            }
            _ => {}
        };
    });

    tymap
}

fn parse_variables<'tcx>(
    a: &'tcx DroplessArena,
    m: &Module,
    tymap: &HashMap<u32, &'tcx TypeDesc<'tcx>>,
) -> &'tcx [(IPtr, Variable<'tcx>)] {
    let vars: Vec<_> = m
        .filter_instructions::<IVariable>()
        .map(|(iptr, v)| {
            (
                iptr,
                Variable {
                    id: v.result_id,
                    ty: tymap[&v.result_type_id],
                    deco: a.alloc_extend(
                        m.filter_instructions::<IDecorate>()
                            .filter(|(_, d)| d.target_id == v.result_id)
                            .map(|(iptr, d)| {
                                (
                                    iptr,
                                    match d.decoration {
                                        Decoration::Block => ParsedDecoration::Block,
                                        Decoration::BufferBlock => ParsedDecoration::BufferBlock,
                                        Decoration::Constant => ParsedDecoration::Constant,
                                        Decoration::Uniform => ParsedDecoration::Uniform,
                                        Decoration::Location => {
                                            ParsedDecoration::Location(d.params[0])
                                        }
                                        Decoration::Index => ParsedDecoration::Index(d.params[0]),
                                        Decoration::Binding => {
                                            ParsedDecoration::Binding(d.params[0])
                                        }
                                        Decoration::DescriptorSet => {
                                            ParsedDecoration::DescriptorSet(d.params[0])
                                        }
                                        other => ParsedDecoration::Other(other),
                                    },
                                )
                            }),
                    ),
                    storage: v.storage_class,
                },
            )
        })
        .collect();
    a.alloc_extend(vars.into_iter())
}
