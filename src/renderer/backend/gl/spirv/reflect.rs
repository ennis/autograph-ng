//use super::parse::SpirvModule;
use super::parse::*;
use crate::renderer::{Format, ImageDataType, PrimitiveType, TypeDesc};
use spirv_headers::Decoration;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use typed_arena::Arena;

pub enum ParsedDecoration {
    Block,
    Constant,
    Location(u32),
    Index(u32),
    Binding(u32),
    DescriptorSet(u32),
    Uniform,
    Other(Decoration),
}

pub struct Arenas<'tcx> {
    tydesc: Arena<TypeDesc<'tcx>>,
    members: Arena<(usize, &'tcx TypeDesc<'tcx>)>,
    deco: Arena<ParsedDecoration>,
    vars: Arena<Variable<'tcx>>,
}

pub struct Variable<'tcx> {
    id: u32,
    ty: &'tcx TypeDesc<'tcx>,
    deco: &'tcx [ParsedDecoration],
}

pub struct Ast<'tcx, 'm> {
    m: &'m mut Module,
    a: &'tcx Arenas<'tcx>,
    tymap: HashMap<u32, &'tcx TypeDesc<'tcx>>,
    // cached: invalidated when modifying spir-v
    vars: Cell<Option<&'tcx [Variable<'tcx>]>>,
}

impl<'tcx, 'm> Ast<'tcx, 'm> {
    pub fn new(arenas: &'tcx Arenas<'tcx>, module: &'m mut Module) -> Ast<'tcx, 'm> {
        let tymap = build_types(arenas, module);
        Ast {
            m: module,
            a: arenas,
            tymap,
            vars: Cell::new(None),
        }
    }

    // Borrow self to avoid modifications.
    // In fact, modifications to the underlying spir-v will not invalidate the
    // variables, since they are allocated in a separate arena, but
    // this is conceptually surprising.
    pub fn global_variables<'a>(&'a self) -> &'a [Variable<'a>] {
        if let Some(ref vars) = self.vars.get() {
            vars
        } else {
            let vars = self
                .a
                .vars
                .alloc_extend(self.m.filter_opcodes::<IVariable>().map(|v| Variable {
                    id: v.result_id,
                    ty: self.tymap[&v.result_type_id],
                    deco: self.decorations_internal(v.result_id),
                }));
            self.vars.set(Some(vars));
            vars
        }
    }

    fn decorations_internal(&self, id: u32) -> &'tcx [ParsedDecoration] {
        self.a.deco.alloc_extend(
            self.m
                .filter_opcodes::<IDecorate>()
                .map(|d| match d.decoration {
                    Decoration::Block => ParsedDecoration::Block,
                    Decoration::Constant => ParsedDecoration::Constant,
                    Decoration::Uniform => ParsedDecoration::Uniform,
                    Decoration::Location => ParsedDecoration::Location(d.params[0]),
                    Decoration::Index => ParsedDecoration::Index(d.params[0]),
                    Decoration::Binding => ParsedDecoration::Binding(d.params[0]),
                    Decoration::DescriptorSet => ParsedDecoration::DescriptorSet(d.params[0]),
                    other => ParsedDecoration::Other(other),
                }),
        )
    }

    // lifetime-restricted version of the above
    pub fn decorations<'a>(&'a self, id: u32) -> &'a [ParsedDecoration] {
        self.decorations_internal(id)
    }

    pub fn remove_decoration(&mut self, id: u32, deco: u32) {
        unimplemented!()
    }

    pub fn add_decoration(&mut self, id: u32, deco: Decoration, params: &[u32]) {
        unimplemented!()
    }
}

fn build_types<'tcx, 'm>(
    a: &'tcx Arenas<'tcx>,
    m: &'m Module,
) -> HashMap<u32, &'tcx TypeDesc<'tcx>> {
    // build a map from id to type
    let mut tymap = HashMap::<u32, &'tcx TypeDesc<'tcx>>::new();

    // can process types in order, since the spec specifies that:
    // "Types are built bottom up: A parameterizing operand in a type must be defined before being used."
    m.decoded_instructions().for_each(|inst| {
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
                sampled_type_id,
                dim,
                depth,
                arrayed,
                ms,
                sampled,
                format,
                access,
            }) => {
                // yeah that's not really enough
                tymap.insert(
                    *result_id,
                    a.tydesc.alloc(TypeDesc::Image(ImageDataType::Float, None)),
                );
            }
            Instruction::TypeSampler(ITypeSampler { result_id }) => unimplemented!(),
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
                length_id,
            }) => {
                let ty = tymap[type_id];
                // TODO eval length
                tymap.insert(*result_id, a.tydesc.alloc(TypeDesc::Array(ty, 0)));
            }
            Instruction::TypeRuntimeArray(ITypeRuntimeArray { result_id, type_id }) => {
                let ty = tymap[type_id];
                tymap.insert(*result_id, a.tydesc.alloc(TypeDesc::Array(ty, 0)));
            }
            Instruction::TypeStruct(ITypeStruct {
                result_id,
                member_types,
            }) => {
                /*tymap.insert(
                    *result_id,
                    a.tydesc.alloc(TypeDesc::Struct(
                        a.members
                            .alloc_extend(member_types.iter().map(|tyid| tymap[tyid])),
                    )),
                );*/
                unimplemented!()
            }
            Instruction::TypeOpaque(ITypeOpaque { result_id, name }) => unimplemented!(),
            Instruction::TypePointer(ITypePointer {
                result_id,
                storage_class,
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
