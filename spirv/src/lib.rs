//! SPIR-V parsing and manipulation utilities.
#[macro_use]
extern crate log;

pub mod ast;
mod decode;
mod edit;
pub mod inst;
pub mod layout;

use spirv_headers::ImageFormat;
use std::cell::RefCell;
use std::fmt;
use std::marker::PhantomData;

//pub use self::inst::*;
//pub use self::edit::*;
pub use self::decode::DecodedInstruction;
pub use self::layout::*;

/// Error that can happen when parsing.
#[derive(Debug, Clone)]
pub enum ParseError {
    MissingHeader,
    WrongHeader,
    IncompleteInstruction,
    UnknownConstant(&'static str, u32),
}

/// Be careful not to mix IPtrs between modules
/// IPtrs are invalidated after the module is edited.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct IPtr<'m>(usize, PhantomData<&'m ()>);

impl<'m> fmt::Debug for IPtr<'m> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IPtr({})", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    pub adds: RefCell<Vec<u32>>,
    pub removals: RefCell<Vec<usize>>,
    pub data: Vec<u32>,
    pub version: (u8, u8),
    pub bound: u32,
}

impl Module {
    pub fn from_bytes(data: &[u8]) -> Result<Module, ParseError> {
        if data.len() < 20 {
            return Err(ParseError::MissingHeader);
        }

        // we need to determine whether we are in big endian order or little endian order depending
        // on the magic number at the start of the file
        let data = if data[0] == 0x07 && data[1] == 0x23 && data[2] == 0x02 && data[3] == 0x03 {
            // big endian
            data.chunks(4)
                .map(|c| {
                    ((c[0] as u32) << 24)
                        | ((c[1] as u32) << 16)
                        | ((c[2] as u32) << 8)
                        | c[3] as u32
                })
                .collect::<Vec<_>>()
        } else if data[3] == 0x07 && data[2] == 0x23 && data[1] == 0x02 && data[0] == 0x03 {
            // little endian
            data.chunks(4)
                .map(|c| {
                    ((c[3] as u32) << 24)
                        | ((c[2] as u32) << 16)
                        | ((c[1] as u32) << 8)
                        | c[0] as u32
                })
                .collect::<Vec<_>>()
        } else {
            return Err(ParseError::MissingHeader);
        };

        Self::from_words(&data)
    }

    pub fn from_words(i: &[u32]) -> Result<Module, ParseError> {
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
            adds: RefCell::new(Vec::new()),
            removals: RefCell::new(Vec::new()),
            version,
            bound: i[3],
            data: i.to_vec(),
        })
    }
}

//--------------------------------------------------------------------------------------------------

/// Primitive SPIR-V data types.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrimitiveType {
    /// 32-bit signed integer
    Int,
    /// 32-bit unsigned integer
    UnsignedInt,
    /// 16-bit half float (unused)
    Half,
    /// 32-bit floating-point value
    Float,
    /// 64-bit floating-point value
    Double,
    /// Boolean
    /// TODO size and alignment?
    Bool,
}

/// Texture basic data type (NOT storage format)
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ImageDataType {
    Float, // and also depth
    Integer,
    UnsignedInteger,
}

/// Describes the memory layout of struct fields.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct StructLayout<'tcx> {
    pub fields: &'tcx [(usize, &'tcx TypeDesc<'tcx>)],
}

/// Describes a data type used inside a SPIR-V shader
/// (e.g. the type of a uniform, or the type of vertex attributes as seen by the shader).
///
/// TypeDescs are slightly different from Formats:
/// the latter describes the precise bit layout, packing, numeric format, and interpretation
/// of individual data elements, while the former describes unpacked data as seen inside shaders.
///
/// For instance, a vertex buffer containing an attribute with format `R16G16B16_UNORM` is unpacked
/// and fed to the vertex shader as a 3-component vector of floats, which is represented by
/// `TypeDesc::Vector(PrimitiveType::Float,3)`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TypeDesc<'tcx> {
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Array type.
    Array(&'tcx TypeDesc<'tcx>, usize),
    /// Vector type (ty,size).
    Vector(PrimitiveType, u8),
    /// Matrix type (ty,rows,cols).
    Matrix(PrimitiveType, u8, u8),
    /// Structure type (array of (offset, type) tuples).
    Struct(StructLayout<'tcx>),
    /// Image type.
    Image(ImageDataType, Option<ImageFormat>),
    /// Combination of an image and sampling information.
    SampledImage(ImageDataType, Option<ImageFormat>),
    Void,
    /// Pointer to data.
    Pointer(&'tcx TypeDesc<'tcx>),
    Unknown,
}

pub const TYPE_FLOAT: TypeDesc = TypeDesc::Primitive(PrimitiveType::Float);
pub const TYPE_INT: TypeDesc = TypeDesc::Primitive(PrimitiveType::Int);
pub const TYPE_VEC2: TypeDesc = TypeDesc::Vector(PrimitiveType::Float, 2);
pub const TYPE_VEC3: TypeDesc = TypeDesc::Vector(PrimitiveType::Float, 3);
pub const TYPE_VEC4: TypeDesc = TypeDesc::Vector(PrimitiveType::Float, 4);
pub const TYPE_IVEC2: TypeDesc = TypeDesc::Vector(PrimitiveType::Int, 2);
pub const TYPE_IVEC3: TypeDesc = TypeDesc::Vector(PrimitiveType::Int, 3);
pub const TYPE_IVEC4: TypeDesc = TypeDesc::Vector(PrimitiveType::Int, 4);
pub const TYPE_MAT2: TypeDesc = TypeDesc::Matrix(PrimitiveType::Float, 2, 2);
pub const TYPE_MAT3: TypeDesc = TypeDesc::Matrix(PrimitiveType::Float, 3, 3);
pub const TYPE_MAT4: TypeDesc = TypeDesc::Matrix(PrimitiveType::Float, 4, 4);
