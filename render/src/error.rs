//! Error type.
//!
//! This is the shared error type for the whole crate.

// TODO it's unclear what's best: a shared error enum like this, or smaller error types for each module

use std::{error, fmt};

#[derive(Clone, Debug)]
pub enum Error {
    OutOfMemory,
    InvalidRenderTarget,
    InvalidSampledImage,
    InvalidStorageImage,
}

impl fmt::Display for Error {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }
}

impl error::Error for Error {}

pub type Result<T> = ::std::result::Result<T, Error>;
