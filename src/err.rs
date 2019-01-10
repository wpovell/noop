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
        match self {
            Error::Arg { reason } => write!(f, "{}", reason),
            Error::Parse { err } | Error::OS { err } => write!(f, "{}", err),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<NulError> for Error {
    fn from(_err: NulError) -> Error {
        Error::Arg {
            reason: "Null character in argument",
        }
    }
}

impl From<nix::Error> for Error {
    fn from(err: nix::Error) -> Error {
        Error::OS { err: err.into() }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Error {
        Error::Parse { err: err.into() }
    }
}
