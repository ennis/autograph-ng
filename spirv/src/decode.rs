// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.
use super::{inst::*, IPtr, Module, ParseError};
use num_traits::FromPrimitive;
use spirv_headers::*;
use std::marker::PhantomData;

impl Module {
    pub fn decode_raw<'a>(&'a self) -> impl Iterator<Item = (IPtr<'a>, RawInstruction)> {
        struct RawInstIter<'m> {
            i: &'m [u32],
            ptr: usize,
        }

        impl<'m> Iterator for RawInstIter<'m> {
            type Item = (IPtr<'m>, RawInstruction<'m>);

            fn next(&mut self) -> Option<(IPtr<'m>, RawInstruction<'m>)> {
                if self.i.len() >= 1 {
                    let (inst, rest) = decode_raw_instruction(self.i).unwrap();
                    let ptr = self.ptr;
                    self.i = rest;
                    self.ptr += inst.word_count as usize;
                    Some((IPtr(ptr, PhantomData), inst))
                } else {
                    None
                }
            }
        }

        // 5 is beginning of instruction stream
        RawInstIter {
            i: &self.data[5..],
            ptr: 5,
        }
    }

    pub fn filter_instructions<'a, T: DecodedInstruction<'a>>(
        &'a self,
    ) -> impl Iterator<Item = (IPtr<'a>, T)> + 'a {
        self.decode_raw().filter_map(|(iptr, inst)| {
            if inst.opcode == T::OPCODE as u16 {
                Some((iptr, T::decode(inst.operands).into()))
            } else {
                None
            }
        })
    }

    pub fn decode(&self) -> impl Iterator<Item = (IPtr, Instruction)> {
        self.decode_raw().map(|(iptr, inst)| (iptr, inst.decode()))
    }

    pub fn decode_raw_at<'a>(&'a self, iptr: IPtr) -> Result<RawInstruction<'a>, ParseError> {
        decode_raw_instruction(&self.data[iptr.0..]).map(|(inst, _)| inst)
    }

    pub fn next_iptr<'a>(&'a self, ptr: IPtr) -> Result<IPtr<'a>, ParseError> {
        Ok(IPtr(
            ptr.0 + self.decode_raw_at(ptr)?.word_count as usize,
            PhantomData,
        ))
    }
}

pub trait DecodedInstruction<'m>: 'm {
    const OPCODE: Op;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self;
    fn encode(&self, _out_instructions: &mut Vec<u32>) {
        unimplemented!()
    }
}

//impl DecodedInstruction for INop { const OPCODE: u16 = 0; }
impl<'m> DecodedInstruction<'m> for IName {
    const OPCODE: Op = Op::Name;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IName {
            target_id: operands[0],
            name: parse_string(&operands[1..]).0,
        }
    }
}
impl<'m> DecodedInstruction<'m> for IMemberName {
    const OPCODE: Op = Op::MemberName;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IMemberName {
            target_id: operands[0],
            member: operands[1],
            name: parse_string(&operands[2..]).0,
        }
    }
}
impl<'m> DecodedInstruction<'m> for IExtInstImport {
    const OPCODE: Op = Op::ExtInstImport;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IExtInstImport {
            result_id: operands[0],
            name: parse_string(&operands[1..]).0,
        }
    }
}
impl<'m> DecodedInstruction<'m> for IMemoryModel {
    const OPCODE: Op = Op::MemoryModel;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IMemoryModel(
            try_parse_constant::<AddressingModel>(operands[0]).unwrap(),
            try_parse_constant::<MemoryModel>(operands[1]).unwrap(),
        )
    }
}
impl<'m> DecodedInstruction<'m> for IEntryPoint<'m> {
    const OPCODE: Op = Op::EntryPoint;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        {
            let (n, r) = parse_string(&operands[2..]);
            IEntryPoint {
                execution: try_parse_constant::<ExecutionModel>(operands[0]).unwrap(),
                id: operands[1],
                name: n,
                interface: r,
            }
        }
    }
}
impl<'m> DecodedInstruction<'m> for IExecutionMode<'m> {
    const OPCODE: Op = Op::ExecutionMode;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IExecutionMode {
            target_id: operands[0],
            mode: try_parse_constant::<ExecutionMode>(operands[1]).unwrap(),
            optional_literals: &operands[2..],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ICapability {
    const OPCODE: Op = Op::Capability;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ICapability(try_parse_constant::<Capability>(operands[0]).unwrap())
    }
}
impl<'m> DecodedInstruction<'m> for ITypeVoid {
    const OPCODE: Op = Op::TypeVoid;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeVoid {
            result_id: operands[0],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeBool {
    const OPCODE: Op = Op::TypeBool;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeBool {
            result_id: operands[0],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeInt {
    const OPCODE: Op = Op::TypeInt;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeInt {
            result_id: operands[0],
            width: operands[1],
            signedness: operands[2] != 0,
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeFloat {
    const OPCODE: Op = Op::TypeFloat;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeFloat {
            result_id: operands[0],
            width: operands[1],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeVector {
    const OPCODE: Op = Op::TypeVector;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeVector {
            result_id: operands[0],
            component_id: operands[1],
            count: operands[2],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeMatrix {
    const OPCODE: Op = Op::TypeMatrix;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeMatrix {
            result_id: operands[0],
            column_type_id: operands[1],
            column_count: operands[2],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeImage {
    const OPCODE: Op = Op::TypeImage;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeImage {
            result_id: operands[0],
            sampled_type_id: operands[1],
            dim: try_parse_constant::<Dim>(operands[2]).unwrap(),
            depth: match operands[3] {
                0 => Some(false),
                1 => Some(true),
                2 => None,
                _ => unreachable!(),
            },
            arrayed: operands[4] != 0,
            ms: operands[5] != 0,
            sampled: match operands[6] {
                0 => None,
                1 => Some(true),
                2 => Some(false),
                _ => unreachable!(),
            },
            format: try_parse_constant::<ImageFormat>(operands[7]).unwrap(),
            access: if operands.len() >= 9 {
                Some(try_parse_constant::<AccessQualifier>(operands[8]).unwrap())
            } else {
                None
            },
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeSampler {
    const OPCODE: Op = Op::TypeSampler;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeSampler {
            result_id: operands[0],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeSampledImage {
    const OPCODE: Op = Op::TypeSampledImage;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeSampledImage {
            result_id: operands[0],
            image_type_id: operands[1],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeArray {
    const OPCODE: Op = Op::TypeArray;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeArray {
            result_id: operands[0],
            type_id: operands[1],
            length_id: operands[2],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeRuntimeArray {
    const OPCODE: Op = Op::TypeRuntimeArray;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeRuntimeArray {
            result_id: operands[0],
            type_id: operands[1],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeStruct<'m> {
    const OPCODE: Op = Op::TypeStruct;
    fn decode<'a: 'm>(operands: &'a [u32]) -> ITypeStruct<'m> {
        ITypeStruct {
            result_id: operands[0],
            member_types: &operands[1..],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypeOpaque {
    const OPCODE: Op = Op::TypeOpaque;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypeOpaque {
            result_id: operands[0],
            name: parse_string(&operands[1..]).0,
        }
    }
}
impl<'m> DecodedInstruction<'m> for ITypePointer {
    const OPCODE: Op = Op::TypePointer;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ITypePointer {
            result_id: operands[0],
            storage_class: try_parse_constant::<StorageClass>(operands[1]).unwrap(),
            type_id: operands[2],
        }
    }
}
impl<'m> DecodedInstruction<'m> for IConstant<'m> {
    const OPCODE: Op = Op::Constant;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IConstant {
            result_type_id: operands[0],
            result_id: operands[1],
            data: &operands[2..],
        }
    }
}
//impl DecodedInstruction<'static for IFunctionEnd { const OPCODE: u16 = 56; }
impl<'m> DecodedInstruction<'m> for IVariable {
    const OPCODE: Op = Op::Variable;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IVariable {
            result_type_id: operands[0],
            result_id: operands[1],
            storage_class: try_parse_constant::<StorageClass>(operands[2]).unwrap(),
            initializer: operands.get(3).map(|&v| v),
        }
    }
}
impl<'m> DecodedInstruction<'m> for IDecorate<'m> {
    const OPCODE: Op = Op::Decorate;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IDecorate {
            target_id: operands[0],
            decoration: try_parse_constant::<Decoration>(operands[1]).unwrap(),
            params: &operands[2..],
        }
    }

    fn encode(&self, out: &mut Vec<u32>) {
        encode_instruction(
            out,
            Op::Decorate,
            [self.target_id, self.decoration as u32]
                .iter()
                .cloned()
                .chain(self.params.iter().cloned()),
        );
    }
}
impl<'m> DecodedInstruction<'m> for IMemberDecorate<'m> {
    const OPCODE: Op = Op::MemberDecorate;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IMemberDecorate {
            target_id: operands[0],
            member: operands[1],
            decoration: try_parse_constant::<Decoration>(operands[2]).unwrap(),
            params: &operands[3..],
        }
    }
}
impl<'m> DecodedInstruction<'m> for ILabel {
    const OPCODE: Op = Op::Label;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        ILabel {
            result_id: operands[0],
        }
    }
}
impl<'m> DecodedInstruction<'m> for IBranch {
    const OPCODE: Op = Op::Branch;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        IBranch {
            result_id: operands[0],
        }
    }
}
/*impl DecodedInstruction for IKill {
    const OPCODE: u16 = 252;
    fn decode(operands: &[u32]) -> Self {
        unimplemented!()
    }
}
impl DecodedInstruction for IReturn {
    const OPCODE: u16 = 253;
    fn decode(operands: &[u32]) -> Self {
        unimplemented!()
    }
}*/

impl<'m> RawInstruction<'m> {
    pub fn decode(&self) -> Instruction<'m> {
        decode_instruction(self.opcode, self.operands).unwrap()
    }
}

pub(super) fn decode_raw_instruction(i: &[u32]) -> Result<(RawInstruction, &[u32]), ParseError> {
    assert!(i.len() >= 1);
    let word_count = (i[0] >> 16) as usize;
    assert!(word_count >= 1);
    let opcode = (i[0] & 0xffff) as u16;

    if i.len() < word_count {
        return Err(ParseError::IncompleteInstruction);
    }

    let raw_inst = RawInstruction {
        opcode,
        word_count: word_count as u16,
        operands: &i[1..word_count],
    };

    Ok((raw_inst, &i[word_count..]))
}

fn try_parse_constant<T: FromPrimitive>(constant: u32) -> Result<T, ParseError> {
    T::from_u32(constant).ok_or(ParseError::UnknownConstant("unknown", constant))
}

fn encode_instruction(out: &mut Vec<u32>, opcode: Op, operands: impl Iterator<Item = u32>) {
    let sptr = out.len();
    out.push(0);
    out.extend(operands);
    let eptr = out.len();
    out[sptr] = (opcode as u32) | ((eptr - sptr) as u32) << 16;
}

fn decode_instruction(opcode: u16, operands: &[u32]) -> Result<Instruction, ParseError> {
    Ok(match opcode {
        0 => Instruction::Nop,
        5 => Instruction::Name(IName::decode(operands)),
        6 => Instruction::MemberName(IMemberName::decode(operands)),
        11 => Instruction::ExtInstImport(IExtInstImport::decode(operands)),
        14 => Instruction::MemoryModel(IMemoryModel::decode(operands)),
        15 => Instruction::EntryPoint(IEntryPoint::decode(operands)),
        16 => Instruction::ExecutionMode(IExecutionMode::decode(operands)),
        17 => Instruction::Capability(ICapability::decode(operands)),
        19 => Instruction::TypeVoid(ITypeVoid::decode(operands)),
        20 => Instruction::TypeBool(ITypeBool::decode(operands)),
        21 => Instruction::TypeInt(ITypeInt::decode(operands)),
        22 => Instruction::TypeFloat(ITypeFloat::decode(operands)),
        23 => Instruction::TypeVector(ITypeVector::decode(operands)),
        24 => Instruction::TypeMatrix(ITypeMatrix::decode(operands)),
        25 => Instruction::TypeImage(ITypeImage::decode(operands)),
        26 => Instruction::TypeSampler(ITypeSampler::decode(operands)),
        27 => Instruction::TypeSampledImage(ITypeSampledImage::decode(operands)),
        28 => Instruction::TypeArray(ITypeArray::decode(operands)),
        29 => Instruction::TypeRuntimeArray(ITypeRuntimeArray::decode(operands)),
        30 => Instruction::TypeStruct(ITypeStruct::decode(operands)),
        31 => Instruction::TypeOpaque(ITypeOpaque::decode(operands)),
        32 => Instruction::TypePointer(ITypePointer::decode(operands)),
        43 => Instruction::Constant(IConstant::decode(operands)),
        56 => Instruction::FunctionEnd,
        59 => Instruction::Variable(IVariable::decode(operands)),
        71 => Instruction::Decorate(IDecorate::decode(operands)),
        72 => Instruction::MemberDecorate(IMemberDecorate::decode(operands)),
        248 => Instruction::Label(ILabel::decode(operands)),
        249 => Instruction::Branch(IBranch::decode(operands)),
        252 => Instruction::Kill,
        253 => Instruction::Return,
        _ => Instruction::Unknown(IUnknownInst(opcode, operands.to_owned())),
    })
}

fn parse_string(data: &[u32]) -> (String, &[u32]) {
    let bytes = data
        .iter()
        .flat_map(|&n| {
            let b1 = (n & 0xff) as u8;
            let b2 = ((n >> 8) & 0xff) as u8;
            let b3 = ((n >> 16) & 0xff) as u8;
            let b4 = ((n >> 24) & 0xff) as u8;
            vec![b1, b2, b3, b4].into_iter()
        })
        .take_while(|&b| b != 0)
        .collect::<Vec<u8>>();

    let r = 1 + bytes.len() / 4;
    let s = String::from_utf8(bytes).expect("Shader content is not UTF-8");

    (s, &data[r..])
}
