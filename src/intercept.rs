//! Code for intercepting and handling child process syscalls

extern crate nix;
use nix::libc::user_regs_struct as Regs;
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::wait::waitpid;
use nix::unistd::{execvp, fork, ForkResult, Pid};

use std::ffi::CString;
use std::fs;
use std::path::PathBuf;
use std::process;

use crate::args::Args;
use crate::child;
use crate::err::Result;
use crate::types::{Action, OpenType};

// Syscall numbers
// TODO: Would be nice to wrap these in an enum somehow
const SYS_OPEN: u64 = 2;
const SYS_OPENAT: u64 = 257;
const SYS_OPEN_BY_HANDLE_AT: u64 = 304;
const SYS_EXIT: u64 = 60;
const SYS_EXIT_GROUP: u64 = 231;

/// Convert syscall number to string name
fn sys_to_name(sys: u64) -> Option<&'static str> {
    match sys {
        SYS_OPEN => Some("open"),
        SYS_OPENAT => Some("openat"),
        SYS_OPEN_BY_HANDLE_AT => Some("open_by_handle_at"),
        _ => None,
    }
}

/// Parse child address holding a `CString` into a `PathBuf`
///
/// This function is marked unsafe as `addr` must be the address of a `CString`
/// or behavior is undefined.
unsafe fn user_path(pid: Pid, addr: u64) -> Result<PathBuf> {
    let path = child::read_data(pid, addr, None)?;
    let path = std::str::from_utf8(&path)?;
    let path = match fs::canonicalize(path) {
        Ok(pathbuf) => pathbuf,
        Err(_) => PathBuf::from(path),
    };

    Ok(path)
}

/// Rewrite `regs` to redirect `open` call to `new` path
///
/// This function extends the child process stack, writes the new path,
/// and updates the path argument in `regs` to point to this new value.
fn redirect_path(pid: Pid, stack: u64, arg: &mut u64, new: &PathBuf) -> Result<()> {
    let mut path = CString::new(new.to_str()?.as_bytes())?.into_bytes_with_nul();
    let file_addr = stack - 128 - path.len() as u64;
    *arg = file_addr;

    child::write_data(pid, file_addr, &mut path)?;

    Ok(())
}

/// Checks if path is allowed for given mode and returns action for it
fn check_open(mode: &OpenType, rule: Option<&Action>) -> bool {
    use crate::types::Action::*;
    match rule {
        Some(Block(OpenType::All)) => false,
        Some(Block(typ)) => *mode == OpenType::All || *typ != *mode,
        Some(Replace(_)) => true,
        None => true,
    }
}

/// Fork child to run passed program and begin tracing
fn trace_child(argv: &[CString]) -> Result<Pid> {
    let pid = match fork()? {
        ForkResult::Parent { child } => child,
        ForkResult::Child => {
            ptrace::traceme()?;
            if execvp(&argv[0], argv).is_err() {
                eprintln!("Failed to execute {:?}", argv[0]);
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
    };

    Ok(pid)
}

/// Handle child call to `open`
fn handle_open(pid: Pid, args: &Args, regs: &mut Regs) -> Result<()> {
    let syscall = regs.orig_rax;

    // Syscall name, address to path, opening mode
    let (path_reg, flag_reg) = match syscall {
        SYS_OPEN => (&mut regs.rdi, regs.rsi),
        SYS_OPENAT => (&mut regs.rsi, regs.rdx),
        SYS_OPEN_BY_HANDLE_AT => (&mut regs.rsi, regs.r8),
        _ => panic!("Bad syscall passed to handle_open"),
    };

    // Read path from child
    let path = unsafe { user_path(pid, *path_reg)? };

    // Parse open mode from flag register
    let mode = OpenType::from(flag_reg);

    // Check if permitted
    let action = args.paths.get(&path);
    let allowed = check_open(&mode, action);

    if args.show {
        // Log open call
        let name = sys_to_name(syscall).unwrap();
        eprint!("{}({:?}, {})", name, path, mode);

        if !allowed {
            eprint!(" BLOCKED");
        } else if let Some(Action::Replace(new)) = action {
            eprint!(" => {}", &new.to_string_lossy());
        }
        eprintln!();
    }

    if let Some(Action::Replace(new)) = action {
        eprintln!("orig: {:?}", path);
        // Rewrite syscall path
        redirect_path(pid, regs.rsp, path_reg, &new)?;

        let new_path = unsafe { user_path(pid, *path_reg)? };
        eprintln!("new: {:?}", new_path);
    }

    if !allowed {
        // Set syscall to invalid value so it fails to open
        regs.orig_rax = -1i64 as u64;
    }

    ptrace::setregs(pid, *regs)?;

    Ok(())
}

/// Start child process and begin intercepting its calls to open
pub fn start(args: &Args) -> Result<()> {
    // Fork off program
    let pid = trace_child(&args.argv)?;

    loop {
        // Syscall entrance
        ptrace::syscall(pid)?;
        waitpid(pid, None)?;

        let mut regs = ptrace::getregs(pid)?;
        let syscall = regs.orig_rax;

        match syscall {
            // Check if open is allowed
            SYS_OPEN | SYS_OPENAT | SYS_OPEN_BY_HANDLE_AT => handle_open(pid, args, &mut regs)?,
            // Child exited, we will also
            SYS_EXIT | SYS_EXIT_GROUP => process::exit(regs.rdi as i32),
            // Don't check non-open calls
            _ => (),
        };

        // Syscall return
        ptrace::syscall(pid)?;
        waitpid(pid, None)?;
    }
}
