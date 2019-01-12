extern crate byteorder;
use byteorder::{ByteOrder, NativeEndian};

extern crate nix;
use nix::sys::ptrace;
use nix::sys::ptrace::AddressType;
use nix::unistd::Pid;

use std::ffi::c_void;
use std::mem::size_of;

use crate::err::Result;

/// Read data starting at `addr` in `pid`'s memory.
///
/// If `n` is `None`, read until first zero.
/// Otherwise, read exactly `n` bytes.
pub fn read_data(pid: Pid, addr: u64, n: Option<usize>) -> Result<Vec<u8>> {
    let mut data: Vec<u8> = Vec::new();
    let mut loc = addr;
    let n = if let Some(n) = n { n } else { std::usize::MAX } as u64;

    // Read string word by word from child memory address
    let read = 0;
    'outer: while (loc - addr) < n {
        let chars_raw = ptrace::read(pid, loc as AddressType)?;
        let mut chars: [u8; 8] = [0; 8];
        NativeEndian::write_i64(&mut chars, chars_raw);

        for char in chars.iter() {
            if *char == 0 || read == n {
                break 'outer;
            }
            data.push(*char)
        }
        loc += size_of::<i64>() as u64;
    }

    Ok(data)
}

/// Write data to `addr` in `pid`'s memory.
///
/// Data is zero-padded if not a multiple of 8.
pub fn write_data(pid: Pid, addr: u64, data: &mut Vec<u8>) -> Result<()> {
    // Pad with zeros
    let chunk_size = size_of::<i64>();
    let padding = chunk_size - (data.len() % chunk_size);
    for _ in 0..padding {
        data.push(0);
    }

    let mut loc = addr;
    for chunk in data.chunks_mut(chunk_size) {
        let chunk = NativeEndian::read_u64(chunk);
        ptrace::write(pid, loc as AddressType, chunk as *mut c_void)?;

        loc += chunk_size as u64;
    }

    Ok(())
}
