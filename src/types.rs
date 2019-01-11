//! Common types used across the crate, including file blocking data

extern crate nix;
use nix::libc::{O_RDWR, O_WRONLY};

use std::fmt;
use std::path::PathBuf;

/// `open` mode to block
#[derive(PartialEq, Debug, Clone)]
pub enum OpenType {
    Read,
    Write,
    All,
}

impl fmt::Display for OpenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::OpenType::*;
        let mode = match *self {
            Read => "R",
            Write => "W",
            All => "RW",
        };

        write!(f, "{}", mode)
    }
}

impl From<u64> for OpenType {
    fn from(mode: u64) -> Self {
        Self::from(mode as i32)
    }
}

impl From<i32> for OpenType {
    fn from(mode: i32) -> Self {
        use self::OpenType::*;
        if mode & O_WRONLY == O_WRONLY {
            Write
        } else if mode & O_RDWR == O_RDWR {
            All
        } else {
            // O_RDONLY = 0, so it is the default
            Read
        }
    }
}

/// Action to take for a given file
#[derive(Debug, Clone)]
pub enum Action {
    Block(OpenType),
    Replace(PathBuf),
}

#[cfg(test)]
mod test {
    use super::OpenType::*;
    use super::*;
    use nix::libc::{O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY};

    /// Test standard `OpenType` parsing
    #[test]
    fn parse() {
        assert_eq!(OpenType::from(O_RDONLY), Read);
        assert_eq!(OpenType::from(O_WRONLY), Write);
        assert_eq!(OpenType::from(O_RDWR), All);
    }

    /// Test optional `open` flags
    #[test]
    fn extra() {
        assert_eq!(OpenType::from(O_RDONLY | O_CREAT | O_TRUNC), Read);
    }
}
