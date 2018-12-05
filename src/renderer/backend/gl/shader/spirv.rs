// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use num_traits::cast::FromPrimitive;
use spirv::*;

/// Parses a SPIR-V document.
pub fn parse_spirv(data: &[u8]) -> Result<SpirvModule, ParseError> {
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
pub fn parse_spirv_u32s(i: &[u32]) -> Result<SpirvModule, ParseError> {
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

    let instructions = {
        let mut ret = Vec::new();
        let mut i = &i[5..];
        while i.len() >= 1 {
            let (instruction, rest) = parse_instruction(i)?;
            ret.push(instruction);
            i = rest;
        }
        ret
    };

    Ok(SpirvModule {
        version: version,
        bound: i[3],
        instructions: instructions,
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
pub struct SpirvModule {
    pub version: (u8, u8),
    pub bound: u32,
    pub instructions: Vec<Instruction>,
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
pub struct IEntryPoint {
    pub execution: ExecutionModel,
    pub id: u32,
    pub name: String,
    pub interface: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct IExecutionMode {
    pub target_id: u32,
    pub mode: ExecutionMode,
    pub optional_literals: Vec<u32>,
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
pub struct ITypeStruct {
    pub result_id: u32,
    pub member_types: Vec<u32>,
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
pub struct IConstant {
    pub result_type_id: u32,
    pub result_id: u32,
    pub data: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct IVariable {
    pub result_type_id: u32,
    pub result_id: u32,
    pub storage_class: StorageClass,
    pub initializer: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct IDecorate {
    pub target_id: u32,
    pub decoration: Decoration,
    pub params: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct IMemberDecorate {
    pub target_id: u32,
    pub member: u32,
    pub decoration: Decoration,
    pub params: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct ILabel {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub struct IBranch {
    pub result_id: u32,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Unknown(IUnknownInst),
    Nop,
    Name(IName),
    MemberName(IMemberName),
    ExtInstImport(IExtInstImport),
    MemoryModel(IMemoryModel),
    EntryPoint(IEntryPoint),
    ExecutionMode(IExecutionMode),
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
    TypeStruct(ITypeStruct),
    TypeOpaque(ITypeOpaque),
    TypePointer(ITypePointer),
    Constant(IConstant),
    FunctionEnd,
    Variable(IVariable),
    Decorate(IDecorate),
    MemberDecorate(IMemberDecorate),
    Label(ILabel),
    Branch(IBranch),
    Kill,
    Return,
}

fn parse_instruction(i: &[u32]) -> Result<(Instruction, &[u32]), ParseError> {
    assert!(i.len() >= 1);

    let word_count = (i[0] >> 16) as usize;
    assert!(word_count >= 1);
    let opcode = (i[0] & 0xffff) as u16;

    if i.len() < word_count {
        return Err(ParseError::IncompleteInstruction);
    }

    let opcode = decode_instruction(opcode, &i[1..word_count])?;
    Ok((opcode, &i[word_count..]))
}

fn decode_instruction(opcode: u16, operands: &[u32]) -> Result<Instruction, ParseError> {
    fn try_parse_constant<T: FromPrimitive>(constant: u32) -> Result<T, ParseError> {
        T::from_u32(constant).ok_or(ParseError::UnknownConstant("unknown", constant))
    }

    Ok(match opcode {
        0 => Instruction::Nop,
        5 => Instruction::Name(IName {
            target_id: operands[0],
            name: parse_string(&operands[1..]).0,
        }),
        6 => Instruction::MemberName(IMemberName {
            target_id: operands[0],
            member: operands[1],
            name: parse_string(&operands[2..]).0,
        }),
        11 => Instruction::ExtInstImport(IExtInstImport {
            result_id: operands[0],
            name: parse_string(&operands[1..]).0,
        }),
        14 => Instruction::MemoryModel(IMemoryModel(
            try_parse_constant::<AddressingModel>(operands[0])?,
            try_parse_constant::<MemoryModel>(operands[1])?,
        )),
        15 => {
            let (n, r) = parse_string(&operands[2..]);
            Instruction::EntryPoint(IEntryPoint {
                execution: try_parse_constant::<ExecutionModel>(operands[0])?,
                id: operands[1],
                name: n,
                interface: r.to_owned(),
            })
        }
        16 => Instruction::ExecutionMode(IExecutionMode {
            target_id: operands[0],
            mode: try_parse_constant::<ExecutionMode>(operands[1])?,
            optional_literals: operands[2..].to_vec(),
        }),
        17 => Instruction::Capability(ICapability(try_parse_constant::<Capability>(operands[0])?)),
        19 => Instruction::TypeVoid(ITypeVoid {
            result_id: operands[0],
        }),
        20 => Instruction::TypeBool(ITypeBool {
            result_id: operands[0],
        }),
        21 => Instruction::TypeInt(ITypeInt {
            result_id: operands[0],
            width: operands[1],
            signedness: operands[2] != 0,
        }),
        22 => Instruction::TypeFloat(ITypeFloat {
            result_id: operands[0],
            width: operands[1],
        }),
        23 => Instruction::TypeVector(ITypeVector {
            result_id: operands[0],
            component_id: operands[1],
            count: operands[2],
        }),
        24 => Instruction::TypeMatrix(ITypeMatrix {
            result_id: operands[0],
            column_type_id: operands[1],
            column_count: operands[2],
        }),
        25 => Instruction::TypeImage(ITypeImage {
            result_id: operands[0],
            sampled_type_id: operands[1],
            dim: try_parse_constant::<Dim>(operands[2])?,
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
            format: try_parse_constant::<ImageFormat>(operands[7])?,
            access: if operands.len() >= 9 {
                Some(try_parse_constant::<AccessQualifier>(operands[8])?)
            } else {
                None
            },
        }),
        26 => Instruction::TypeSampler(ITypeSampler {
            result_id: operands[0],
        }),
        27 => Instruction::TypeSampledImage(ITypeSampledImage {
            result_id: operands[0],
            image_type_id: operands[1],
        }),
        28 => Instruction::TypeArray(ITypeArray {
            result_id: operands[0],
            type_id: operands[1],
            length_id: operands[2],
        }),
        29 => Instruction::TypeRuntimeArray(ITypeRuntimeArray {
            result_id: operands[0],
            type_id: operands[1],
        }),
        30 => Instruction::TypeStruct(ITypeStruct {
            result_id: operands[0],
            member_types: operands[1..].to_owned(),
        }),
        31 => Instruction::TypeOpaque(ITypeOpaque {
            result_id: operands[0],
            name: parse_string(&operands[1..]).0,
        }),
        32 => Instruction::TypePointer(ITypePointer {
            result_id: operands[0],
            storage_class: try_parse_constant::<StorageClass>(operands[1])?,
            type_id: operands[2],
        }),
        43 => Instruction::Constant(IConstant {
            result_type_id: operands[0],
            result_id: operands[1],
            data: operands[2..].to_owned(),
        }),
        56 => Instruction::FunctionEnd,
        59 => Instruction::Variable(IVariable {
            result_type_id: operands[0],
            result_id: operands[1],
            storage_class: try_parse_constant::<StorageClass>(operands[2])?,
            initializer: operands.get(3).map(|&v| v),
        }),
        71 => Instruction::Decorate(IDecorate {
            target_id: operands[0],
            decoration: try_parse_constant::<Decoration>(operands[1])?,
            params: operands[2..].to_owned(),
        }),
        72 => Instruction::MemberDecorate(IMemberDecorate {
            target_id: operands[0],
            member: operands[1],
            decoration: try_parse_constant::<Decoration>(operands[2])?,
            params: operands[3..].to_owned(),
        }),
        248 => Instruction::Label(ILabel {
            result_id: operands[0],
        }),
        249 => Instruction::Branch(IBranch {
            result_id: operands[0],
        }),
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
