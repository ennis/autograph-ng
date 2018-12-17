mod ast;
mod decode;
mod inst;
mod edit;

use std::marker::PhantomData;
use std::cell::RefCell;

pub use self::ast::*;
pub use self::decode::*;
pub use self::inst::*;
pub use self::edit::*;

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
#[derive(Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd)]
pub struct IPtr<'m>(usize, PhantomData<&'m ()>);

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
