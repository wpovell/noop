extern crate nix;
use nix::errno::Errno;
use nix::libc::user_regs_struct;
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::wait::waitpid;
use nix::unistd::{execvp, fork, ForkResult, Pid};

use std::ffi::c_void;
use std::path::PathBuf;
use std::process;

use crate::args::{Action, Args};
use crate::err::Result;

const SYS_OPEN: u64 = 2;
const SYS_OPENAT: u64 = 257;
const SYS_OPEN_BY_HANDLE_AT: u64 = 304;
const SYS_EXIT: u64 = 60;
const SYS_EXIT_GROUP: u64 = 231;

/// Parse pid address into a path
///
/// This function is marked unsafe as `addr` must be the address of a CString
unsafe fn user_path(pid: Pid, addr: u64) -> Result<PathBuf> {
    let mut path: Vec<u8> = Vec::new();
    let mut loc = addr;

    // Read string word by word from child memory address
    'outer: loop {
        let chars = ptrace::read(pid, loc as *mut c_void)?;
        let chars: [u8; 8] = std::mem::transmute(chars);
        for char in chars.iter() {
            if *char != 0 {
                path.push(*char)
            } else {
                // Found null-terminator, we're done
                break 'outer;
            }
        }
        loc += std::mem::size_of::<i64>() as u64;
    }
    let path = std::str::from_utf8(&path)?;
    Ok(PathBuf::from(path))
}

/// Handle an open call.
///
/// Mutates registers and returns false if call is to be blocked
fn handle_open(pid: Pid, syscall: u64, regs: &mut user_regs_struct, args: &Args) -> Result<bool> {
    let (name, path_reg, flag_reg) = match syscall {
        SYS_OPEN => ("open", regs.rdi, regs.rsi),
        SYS_OPENAT => ("openat", regs.rsi, regs.rdx),
        SYS_OPEN_BY_HANDLE_AT => ("open_by_handle_at", regs.rsi, regs.r8),
        _ => panic!("Bad syscall passed to handle_open"),
    };

    // Read path from child process
    let path = unsafe { user_path(pid, path_reg)? };

    let mut ret = true;
    if let Some(Action::Block(_)) = args.paths.get(&path) {
        // Set syscall to invalid value so it fails to open
        regs.orig_rax = -1i64 as u64;
        ptrace::setregs(pid, *regs)?;
        ret = false;
    }

    if args.show {
        // Print out open call
        eprint!("{}({:?})", name, path);
        if !ret {
            eprint!(" BLOCKED");
        }
        eprintln!();
    }

    Ok(ret)
}

/// Start child process and begin intercepting its calls to open
pub fn start(args: &Args) -> Result<()> {
    // Fork off child program
    let pid = match fork()? {
        ForkResult::Parent { child } => child,
        ForkResult::Child => {
            ptrace::traceme()?;
            if execvp(&args.argv[0], &args.argv).is_err() {
                eprintln!("Failed to execute {:?}", args.argv[0]);
            }
            process::exit(1);
        }
    };

    // Sync with child traceme
    waitpid(pid, None)?;

    // Kill child if we die
    let mut options = Options::empty();
    options.insert(Options::PTRACE_O_EXITKILL);
    if ptrace::setoptions(pid, options).is_err() {
        eprintln!("Failed to trace child");
        process::exit(1);
    }

    loop {
        // Syscall entrance
        ptrace::syscall(pid)?;
        waitpid(pid, None)?;

        let mut regs = ptrace::getregs(pid)?;
        let syscall = regs.orig_rax;

        let allowed = match syscall {
            // Check if open is allowed
            SYS_OPEN | SYS_OPENAT | SYS_OPEN_BY_HANDLE_AT => {
                handle_open(pid, syscall, &mut regs, &args)?
            }
            // Child exited, we will also
            SYS_EXIT | SYS_EXIT_GROUP => process::exit(regs.rdi as i32),
            _ => true,
        };

        // Syscall exit
        ptrace::syscall(pid)?;
        waitpid(pid, None)?;

        // Update return value if blocked
        if !allowed {
            regs.rax = (-(Errno::EPERM as i64)) as u64;
            ptrace::setregs(pid, regs)?;
        }
    }
}
