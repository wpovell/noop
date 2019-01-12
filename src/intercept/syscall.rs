#![allow(non_upper_case_globals)]
extern crate nix;
use nix::libc::user_regs_struct as Regs;
use nix::libc::{SYS_open, SYS_openat};

use std::fmt;

/// Syscalls used by handler
pub enum Syscall {
    Open = SYS_open as isize,
    OpenAt = SYS_openat as isize,
}

impl fmt::Display for Syscall {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Syscall::Open => write!(f, "open"),
            Syscall::OpenAt => write!(f, "openat"),
        }
    }
}

impl Syscall {
    pub fn path<'a>(&self, regs: &'a mut Regs) -> &'a mut u64 {
        use self::Syscall::*;
        match self {
            Open => &mut regs.rdi,
            OpenAt => &mut regs.rsi,
        }
    }

    pub fn flag(&self, regs: &Regs) -> u64 {
        use self::Syscall::*;
        match *self {
            Open => regs.rsi,
            OpenAt => regs.rdx,
        }
    }

    pub fn from(d: u64) -> Syscall {
        use self::Syscall::*;
        match d as i64 {
            SYS_open => Open,
            SYS_openat => OpenAt,
            _ => panic!("No mapping from primitive to Syscall"),
        }
    }
}
