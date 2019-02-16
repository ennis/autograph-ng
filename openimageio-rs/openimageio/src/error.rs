use crate::cstring_to_owned;
use std::error;
use std::fmt;

#[derive(Clone, Debug)]
pub enum Error {
    OpenError(String),
    WriteError(String),
    ReadError(String),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::OpenError(_) => "error opening image",
            Error::WriteError(_) => "error writing image data",
            _ => "unknown error",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::OpenError(ref msg) => write!(f, "Error opening image: {}", msg),
            Error::WriteError(ref msg) => write!(f, "Error writing image data: {}", msg),
            _ => write!(f, "Unknown error."),
        }
    }
}

pub fn get_last_error() -> String {
    unsafe { cstring_to_owned(openimageio_sys::OIIO_geterror()) }
}
