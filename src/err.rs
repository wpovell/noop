use std::error;
use std::ffi::NulError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Arg { reason: &'static str },
    Parse { err: Box<dyn error::Error> },
    OS { err: Box<dyn error::Error> },
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            Arg { reason } => write!(f, "{}", reason),
            Parse { err } | OS { err } => write!(f, "{}", err),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // TODO: Fix this (although I don't think it matters)
        None
    }
}

impl From<NulError> for Error {
    fn from(_err: NulError) -> Self {
        Error::Arg {
            reason: "Null character in argument",
        }
    }
}

impl From<nix::Error> for Error {
    fn from(err: nix::Error) -> Self {
        Error::OS { err: err.into() }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::Parse { err: err.into() }
    }
}
