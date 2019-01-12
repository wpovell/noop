//! Code for intercepting and handling child process syscalls

extern crate nix;
use nix::libc::user_regs_struct as Regs;
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::waitpid;
use nix::unistd::{execvp, fork, getpid, ForkResult, Pid};

use std::ffi::CString;
use std::fs;
use std::path::PathBuf;
use std::process;

use crate::args::Args;
use crate::err::Result;
use crate::types::{Action, OpenType};

mod child;
mod syscall;
use self::syscall::Syscall;
mod seccomp;
use self::seccomp::Context;

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

/// Rewrite `arg` to redirect `open` call to `new` path
///
/// This function extends the child process stack, writes the new path,
/// and updates the path argument in `arg` to point to this new value.
fn redirect_path(pid: Pid, stack: u64, arg: &mut u64, new: &PathBuf) -> Result<()> {
    let mut path = CString::new(new.to_str()?.as_bytes())?.into_bytes_with_nul();

    // Place string below 128B redzone
    let file_addr = stack - 128 - path.len() as u64;

    child::write_data(pid, file_addr, &mut path)?;

    // Update register
    *arg = file_addr;

    Ok(())
}

/// Fork child to run passed program and begin tracing
fn trace_child(argv: &[CString]) -> Result<Pid> {
    let pid = match fork()? {
        ForkResult::Parent { child } => child,
        ForkResult::Child => {
            ptrace::traceme()?;

            // Create seccomp filter
            Context::new()?
                .trace(Syscall::Open as i32)?
                .trace(Syscall::OpenAt as i32)?
                .load()?;

            // TODO: Is this necessary?
            kill(getpid(), Signal::SIGSTOP)?;

            // Execute program
            if execvp(&argv[0], argv).is_err() {
                eprintln!("Failed to execute {:?}", argv[0]);
            }
            process::exit(1);
        }
    };

    // Sync with child traceme
    waitpid(pid, None)?;

    let mut options = Options::empty();
    // Kill child if we die
    options.insert(Options::PTRACE_O_EXITKILL);
    // Catch seccomp filter
    options.insert(Options::PTRACE_O_TRACESECCOMP);
    if ptrace::setoptions(pid, options).is_err() {
        eprintln!("Failed to trace child");
        process::exit(1);
    };

    Ok(pid)
}

/// Handle child call to `open`
fn handle_open(pid: Pid, args: &Args, regs: &mut Regs) -> Result<()> {
    let sys = Syscall::from(regs.orig_rax);

    // Read path from child
    let path = unsafe { user_path(pid, *sys.path(regs))? };

    // Parse open mode from flag register
    let mode = OpenType::from(sys.flag(regs));

    // Check if permitted
    let action = args.paths.get(&path);
    let allowed = action.as_ref().map_or(true, |a| a.allows(&mode));

    if args.show {
        // Log open call
        eprint!("{}({:?}, {})", sys, path, mode);

        if !allowed {
            eprint!(" BLOCKED");
        } else if let Some(Action::Replace(new)) = action {
            eprint!(" => {}", &new.to_string_lossy());
        }
        eprintln!();
    }

    if let Some(Action::Replace(new)) = action {
        // Rewrite syscall path
        redirect_path(pid, regs.rsp, sys.path(regs), &new)?;
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

    let mut handled = 0;
    loop {
        // Syscall entrance
        ptrace::cont(pid, None)?;

        use nix::sys::wait::WaitStatus::*;
        match waitpid(pid, None)? {
            Exited(_, code) => {
                if args.show {
                    eprintln!("\nSUMMARY:\n{} open calls handled", handled);
                }
                process::exit(code);
            }
            PtraceEvent(_, Signal::SIGTRAP, _) => {
                handled += 1;
                let mut regs = ptrace::getregs(pid)?;
                handle_open(pid, args, &mut regs)?;
            }
            _ => (),
        }
    }
}
