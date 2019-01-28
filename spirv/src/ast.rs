//use super::parse::SpirvModule;
use crate::inst::*;
use crate::layout::Std140LayoutBuilder;
use crate::IPtr;
use crate::ImageDataType;
use crate::Module;
use crate::PrimitiveType;
use crate::StructLayout;
use crate::TypeDesc;
use spirv_headers::*;
use std::collections::HashMap;
use typed_arena::Arena;
use crate::StructLayout;

#[derive(Debug)]
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

pub struct Arenas<'tcx, 'm> {
    tydesc: Arena<TypeDesc<'tcx>>,
    members: Arena<(usize, &'tcx TypeDesc<'tcx>)>,
    deco: Arena<(IPtr<'m>, ParsedDecoration)>,
    vars: Arena<(IPtr<'m>, Variable<'tcx, 'm>)>,
}

impl<'tcx, 'm> Arenas<'tcx, 'm> {
    pub fn new() -> Arenas<'tcx, 'm> {
        Arenas {
            tydesc: Arena::new(),
            members: Arena::new(),
            deco: Arena::new(),
            vars: Arena::new(),
        }
    }
}

#[derive(Debug)]
pub struct Variable<'tcx, 'm> {
    pub id: u32,
    pub ty: &'tcx TypeDesc<'tcx>,
    pub deco: &'tcx [(IPtr<'m>, ParsedDecoration)],
    pub storage: StorageClass,
}

impl<'tcx, 'm> Variable<'tcx, 'm> {
    pub fn has_block_decoration(&self) -> Option<IPtr<'m>> {
        self.deco
            .iter()
            .find(|(_, d)| match d {
                ParsedDecoration::Block => true,
                _ => false,
            })
            .map(|d| d.0)
    }

    pub fn has_buffer_block_decoration(&self) -> Option<IPtr<'m>> {
        self.deco
            .iter()
            .find(|(_, d)| match d {
                ParsedDecoration::BufferBlock => true,
                _ => false,
            })
            .map(|d| d.0)
    }

    pub fn descriptor_set_decoration(&self) -> Option<(IPtr<'m>, u32)> {
        self.deco
            .iter()
            .filter_map(|(iptr, d)| match d {
                ParsedDecoration::DescriptorSet(ds) => Some((*iptr, *ds)),
                _ => None,
            })
            .next()
    }

    pub fn binding_decoration(&self) -> Option<(IPtr<'m>, u32)> {
        self.deco
            .iter()
            .filter_map(|(iptr, d)| match d {
                ParsedDecoration::Binding(ds) => Some((*iptr, *ds)),
                _ => None,
            })
            .next()
    }
}

pub struct Ast<'tcx, 'm> {
    //a: &'tcx Arenas<'tcx>,
    _m: &'m Module,
    _tymap: HashMap<u32, &'tcx TypeDesc<'tcx>>,
    vars: &'tcx [(IPtr<'m>, Variable<'tcx, 'm>)],
}

impl<'tcx, 'm> Ast<'tcx, 'm> {
    pub fn new(arenas: &'tcx Arenas<'tcx, 'm>, module: &'m Module) -> Ast<'tcx, 'm> {
        let tymap = parse_types(arenas, module);
        let vars = parse_variables(arenas, module, &tymap);
        Ast {
            _m: module,
            _tymap: tymap,
            vars,
        }
    }

    pub fn variables(&self) -> impl Iterator<Item = &'tcx (IPtr<'m>, Variable<'tcx, 'm>)> {
        self.vars.iter()
    }

    //pub fn uniform_buffers(&self)
}

fn parse_types<'tcx, 'm>(
    a: &'tcx Arenas<'tcx, 'm>,
    m: &Module,
) -> HashMap<u32, &'tcx TypeDesc<'tcx>> {
    // build a map from id to type
    let mut tymap = HashMap::<u32, &'tcx TypeDesc<'tcx>>::new();
    //let mut cstmap = HashMap::<>

    // can process types in order, since the spec specifies that:
    // "Types are built bottom up: A parameterizing operand in a type must be defined before being used."
    m.decode().for_each(|(_, inst)| {
        match &inst {
            Instruction::TypeVoid(ITypeVoid { result_id }) => {
                tymap.insert(*result_id, a.tydesc.alloc(TypeDesc::Void));
            }
            Instruction::TypeBool(ITypeBool { result_id }) => {
                tymap.insert(
                    *result_id,
                    a.tydesc.alloc(TypeDesc::Primitive(PrimitiveType::Bool)),
                );
            }
            Instruction::TypeInt(ITypeInt {
                result_id,
                width,
                signedness,
            }) => {
                assert_eq!(*width, 32, "unsupported bit width");
                match signedness {
                    true => tymap.insert(
                        *result_id,
                        a.tydesc.alloc(TypeDesc::Primitive(PrimitiveType::Int)),
                    ),
                    false => tymap.insert(
                        *result_id,
                        a.tydesc
                            .alloc(TypeDesc::Primitive(PrimitiveType::UnsignedInt)),
                    ),
                };
            }
            Instruction::TypeFloat(ITypeFloat { result_id, width }) => {
                assert_eq!(*width, 32, "unsupported bit width");
                tymap.insert(
                    *result_id,
                    a.tydesc.alloc(TypeDesc::Primitive(PrimitiveType::Float)),
                );
            }
            Instruction::TypeVector(ITypeVector {
                result_id,
                component_id,
                count,
            }) => {
                let compty = tymap[component_id];
                let compty = if let &TypeDesc::Primitive(primty) = &*compty {
                    primty
                } else {
                    panic!("expected primitive type");
                };
                tymap.insert(
                    *result_id,
                    a.tydesc.alloc(TypeDesc::Vector(compty, *count as u8)),
                );
            }
            Instruction::TypeMatrix(ITypeMatrix {
                result_id,
                column_type_id,
                column_count,
            }) => {
                let colty = tymap[column_type_id];
                let (elemty, rows) = if let &TypeDesc::Vector(primty, count) = colty {
                    (primty, count)
                } else {
                    panic!("expected vector type");
                };
                tymap.insert(
                    *result_id,
                    a.tydesc
                        .alloc(TypeDesc::Matrix(elemty, rows, *column_count as u8)),
                );
            }
            Instruction::TypeImage(ITypeImage {
                result_id,
                sampled_type_id: _,
                dim: _,
                depth: _,
                arrayed: _,
                ms: _,
                sampled: _,
                format: _,
                access: _,
            }) => {
                // yeah that's not really enough
                tymap.insert(
                    *result_id,
                    a.tydesc.alloc(TypeDesc::Image(ImageDataType::Float, None)),
                );
            }
            Instruction::TypeSampler(ITypeSampler { result_id: _ }) => unimplemented!(),
            Instruction::TypeSampledImage(ITypeSampledImage {
                result_id,
                image_type_id,
            }) => {
                let imgty = tymap[image_type_id];
                let (dataty, fmt) = if let &TypeDesc::Image(dataty, fmt) = imgty {
                    (dataty, fmt)
                } else {
                    panic!("expected image type")
                };
                tymap.insert(
                    *result_id,
                    a.tydesc.alloc(TypeDesc::SampledImage(dataty, fmt)),
                );
            }
            Instruction::TypeArray(ITypeArray {
                result_id,
                type_id,
                length_id: _,
            }) => {
                let ty = tymap[type_id];
                // TODO eval length
                let stride = Std140AlignAndSize::of(ty).size;
                tymap.insert(*result_id, a.tydesc.alloc(TypeDesc::Array(ty, 0, stride)));
            }
            Instruction::TypeRuntimeArray(ITypeRuntimeArray { result_id, type_id }) => {
                let ty = tymap[type_id];
                let stride = Std140AlignAndSize::of(ty).size;
                tymap.insert(*result_id, a.tydesc.alloc(TypeDesc::Array(ty, 0, stride)));
            }
            Instruction::TypeStruct(ITypeStruct {
                result_id,
                member_types,
            }) => {
                let mut layout_builder = Std140LayoutBuilder::new();
                tymap.insert(
                    *result_id,
                    a.tydesc.alloc(TypeDesc::Struct(StructLayout{
                        fields: a.members.alloc_extend(
                        member_types.iter().map(|tyid| {
                            let ty = tymap[tyid];
                            (layout_builder.add_member(ty), ty)
                        }),
                    )})),
                );
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
                tymap.insert(*result_id, a.tydesc.alloc(TypeDesc::Pointer(ty)));
            }
            _ => {}
        };
    });

    tymap
}

fn parse_variables<'tcx, 'm>(
    a: &'tcx Arenas<'tcx, 'm>,
    m: &'m Module,
    tymap: &HashMap<u32, &'tcx TypeDesc<'tcx>>,
) -> &'tcx [(IPtr<'m>, Variable<'tcx, 'm>)] {
    let vars = a
        .vars
        .alloc_extend(m.filter_instructions::<IVariable>().map(|(iptr, v)| {
            (
                iptr,
                Variable {
                    id: v.result_id,
                    ty: tymap[&v.result_type_id],
                    deco: a.deco.alloc_extend(
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
        }));
    vars
}
