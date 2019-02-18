use crate::cstring_to_owned;
use std::error;
use std::fmt;

#[derive(Clone, Debug)]
pub enum Error {
    OpenError(String),
    WriteError(String),
    ReadError(String),
    BufferTooSmall { len: usize, expected: usize },
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::OpenError(ref msg) => write!(f, "Error opening image: {}", msg),
            Error::WriteError(ref msg) => write!(f, "Error writing image data: {}", msg),
            Error::ReadError(ref msg) => write!(f, "Error reading image data: {}", msg),
            Error::BufferTooSmall { len, expected } => write!(
                f,
                "Buffer was too small (len = {}, expected = {})",
                len, expected
            ),
            //_ => write!(f, "Unknown error."),
        }
    }
}

pub fn get_last_error() -> String {
    unsafe { cstring_to_owned(openimageio_sys::OIIO_geterror()) }
}
