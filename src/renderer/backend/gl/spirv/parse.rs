// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.
use num_traits::FromPrimitive;
use spirv_headers::*;

/// Parses a SPIR-V document.
pub fn parse_spirv(data: &[u8]) -> Result<Module, ParseError> {
    if data.len() < 20 {
        return Err(ParseError::MissingHeader);
    }

    // we need to determine whether we are in big endian order or little endian order depending
    // on the magic number at the start of the file
    let data = if data[0] == 0x07 && data[1] == 0x23 && data[2] == 0x02 && data[3] == 0x03 {
        // big endian
        data.chunks(4)
            .map(|c| {
                ((c[0] as u32) << 24) | ((c[1] as u32) << 16) | ((c[2] as u32) << 8) | c[3] as u32
            })
            .collect::<Vec<_>>()
    } else if data[3] == 0x07 && data[2] == 0x23 && data[1] == 0x02 && data[0] == 0x03 {
        // little endian
        data.chunks(4)
            .map(|c| {
                ((c[3] as u32) << 24) | ((c[2] as u32) << 16) | ((c[1] as u32) << 8) | c[0] as u32
            })
            .collect::<Vec<_>>()
    } else {
        return Err(ParseError::MissingHeader);
    };

    parse_spirv_u32s(&data)
}

/// Parses a SPIR-V document from a list of u32s.
///
/// Endianess has already been handled.
pub fn parse_spirv_u32s(i: &[u32]) -> Result<Module, ParseError> {
    if i.len() < 5 {
        return Err(ParseError::MissingHeader);
    }

    if i[0] != 0x07230203 {
        return Err(ParseError::WrongHeader);
    }

    let version = (
        ((i[1] & 0x00ff0000) >> 16) as u8,
        ((i[1] & 0x0000ff00) >> 8) as u8,
    );

    Ok(Module {
        version: version,
        bound: i[3],
        data: i.to_vec(),
    })
}

/// Error that can happen when parsing.
#[derive(Debug, Clone)]
pub enum ParseError {
    MissingHeader,
    WrongHeader,
    IncompleteInstruction,
    UnknownConstant(&'static str, u32),
}

#[derive(Debug, Clone)]
pub struct Module {
    pub data: Vec<u32>,
    pub version: (u8, u8),
    pub bound: u32,
}

impl Module {
    pub fn raw_instructions(&self) -> impl Iterator<Item = RawInstruction> {
        struct RawInstIter<'m> {
            i: &'m [u32],
        }

        impl<'m> Iterator for RawInstIter<'m> {
            type Item = RawInstruction<'m>;

            fn next(&mut self) -> Option<RawInstruction<'m>> {
                if self.i.len() >= 1 {
                    let (instruction, rest) = parse_raw_instruction(self.i).unwrap();
                    self.i = rest;
                    Some(instruction)
                } else {
                    None
                }
            }
        }

        RawInstIter { i: &self.data }
    }

    pub fn filter_opcodes<'a, T: DecodedInstruction<'a>>(&'a self) -> impl Iterator<Item = T> + 'a {
        self.raw_instructions().filter_map(|inst| {
            if inst.opcode == T::OPCODE as u16 {
                T::decode(inst.operands).into()
            } else {
                None
            }
        })
    }

    pub fn decoded_instructions(&self) -> impl Iterator<Item = Instruction> {
        self.raw_instructions().map(|inst| inst.decode())
    }
}

pub trait DecodedInstruction<'m>: 'm {
    const OPCODE: Op;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self;
}

#[derive(Debug, Clone)]
pub struct IUnknownInst(pub u16, pub Vec<u32>);

#[derive(Debug, Clone)]
pub struct IName {
    pub target_id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct IMemberName {
    pub target_id: u32,
    pub member: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct IExtInstImport {
    pub result_id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct IMemoryModel(pub AddressingModel, pub MemoryModel);

#[derive(Debug, Clone)]
pub struct IEntryPoint<'m> {
    pub execution: ExecutionModel,
    pub id: u32,
    pub name: String,
    pub interface: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct IExecutionMode<'m> {
    pub target_id: u32,
    pub mode: ExecutionMode,
    pub optional_literals: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct ICapability(pub Capability);

#[derive(Debug, Clone)]
pub struct ITypeVoid {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeBool {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeInt {
    pub result_id: u32,
    pub width: u32,
    pub signedness: bool,
}

#[derive(Debug, Clone)]
pub struct ITypeFloat {
    pub result_id: u32,
    pub width: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeVector {
    pub result_id: u32,
    pub component_id: u32,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeMatrix {
    pub result_id: u32,
    pub column_type_id: u32,
    pub column_count: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeImage {
    pub result_id: u32,
    pub sampled_type_id: u32,
    pub dim: Dim,
    pub depth: Option<bool>,
    pub arrayed: bool,
    pub ms: bool,
    pub sampled: Option<bool>,
    pub format: ImageFormat,
    pub access: Option<AccessQualifier>,
}

#[derive(Debug, Clone)]
pub struct ITypeSampler {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeSampledImage {
    pub result_id: u32,
    pub image_type_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeArray {
    pub result_id: u32,
    pub type_id: u32,
    pub length_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeRuntimeArray {
    pub result_id: u32,
    pub type_id: u32,
}

#[derive(Debug, Clone)]
pub struct ITypeStruct<'m> {
    pub result_id: u32,
    pub member_types: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct ITypeOpaque {
    pub result_id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ITypePointer {
    pub result_id: u32,
    pub storage_class: StorageClass,
    pub type_id: u32,
}

#[derive(Debug, Clone)]
pub struct IConstant<'m> {
    pub result_type_id: u32,
    pub result_id: u32,
    pub data: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct IVariable {
    pub result_type_id: u32,
    pub result_id: u32,
    pub storage_class: StorageClass,
    pub initializer: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct IDecorate<'m> {
    pub target_id: u32,
    pub decoration: Decoration,
    pub params: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct IMemberDecorate<'m> {
    pub target_id: u32,
    pub member: u32,
    pub decoration: Decoration,
    pub params: &'m [u32],
}

#[derive(Debug, Clone)]
pub struct ILabel {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct IBranch {
    pub result_id: u32,
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
        unimplemented!()
    }
}
impl<'m> DecodedInstruction<'m> for IBranch {
    const OPCODE: Op = Op::Branch;
    fn decode<'a: 'm>(operands: &'a [u32]) -> Self {
        unimplemented!()
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

#[derive(Debug, Clone)]
pub enum Instruction<'m> {
    Unknown(IUnknownInst),
    Nop,
    Name(IName),
    MemberName(IMemberName),
    ExtInstImport(IExtInstImport),
    MemoryModel(IMemoryModel),
    EntryPoint(IEntryPoint<'m>),
    ExecutionMode(IExecutionMode<'m>),
    Capability(ICapability),
    TypeVoid(ITypeVoid),
    TypeBool(ITypeBool),
    TypeInt(ITypeInt),
    TypeFloat(ITypeFloat),
    TypeVector(ITypeVector),
    TypeMatrix(ITypeMatrix),
    TypeImage(ITypeImage),
    TypeSampler(ITypeSampler),
    TypeSampledImage(ITypeSampledImage),
    TypeArray(ITypeArray),
    TypeRuntimeArray(ITypeRuntimeArray),
    TypeStruct(ITypeStruct<'m>),
    TypeOpaque(ITypeOpaque),
    TypePointer(ITypePointer),
    Constant(IConstant<'m>),
    FunctionEnd,
    Variable(IVariable),
    Decorate(IDecorate<'m>),
    MemberDecorate(IMemberDecorate<'m>),
    Label(ILabel),
    Branch(IBranch),
    Kill,
    Return,
}

pub struct RawInstruction<'m> {
    pub opcode: u16,
    pub word_count: u16,
    pub operands: &'m [u32],
}

impl<'m> RawInstruction<'m> {
    pub fn decode(&self) -> Instruction<'m> {
        decode_instruction(self.opcode, self.operands).unwrap()
    }
}

fn parse_raw_instruction(i: &[u32]) -> Result<(RawInstruction, &[u32]), ParseError> {
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
