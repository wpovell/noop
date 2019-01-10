extern crate nix;
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::errno::Errno;
use nix::sys::wait::waitpid;
use nix::unistd::{Pid, execvp, fork, ForkResult};
use nix::libc::user_regs_struct;

use std::collections::HashMap;
use std::env;
use std::ffi::{c_void, CString};
use std::fmt;
use std::path::PathBuf;
use std::process;

const SYS_OPEN: u64 = 2;
const SYS_OPENAT: u64 = 257;
const SYS_OPEN_BY_HANDLE_AT: u64 = 304;
const SYS_EXIT: u64 = 60;
const SYS_EXIT_GROUP: u64 = 231;

/// Open mode to block
enum OpenType {
    Read,
    Write,
    All,
}

/// Action to take for a given file
enum Action {
    Block(OpenType),
    Replace(PathBuf),
}

/// Print usage message and exit
fn usage(code: i32) -> ! {
    let msg = "noop blocks or modifies calls to open made by the passed program.

USAGE:
  noop [-lh] [FILE[:rw] | FILE=REPLACE]... -- PROGRAM [ARGS...]

FLAGS:
  -l Logs open calls and resulting action to stderr
  -h Show this message and exit

ARGS:
  FILE          Block PROGRAM from opening FILE
      [:rw]     If :r or :w is specified only that opening mode is blocked
  FILE=REPLACE  Replace open calls to FILE with REPLACE
  PROGRAM       PROGRAM to run and intercept on
  ARGS          ARGS to pass to the PROGRAM
";

    eprintln!("{}", msg);
    process::exit(code);
}

/// Wrapper for arugments passed to program
struct Args {
    paths: HashMap<PathBuf, Action>,
    show: bool,
    argv: Vec<CString>,
}

impl fmt::Debug for Args {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "show: {}", self.show)?;
        writeln!(f, "args: {:?}", self.argv)?;
        writeln!(f, "paths:")?;
        for (path, action) in &self.paths {
            write!(f, "\t{:?} => ", path)?;
            match action {
                Action::Block(_) => writeln!(f, "BLOCK"),
                Action::Replace(p) => writeln!(f, "{:?}", p),
            }?;
        }
        Ok(())
    }
}

/// Parse env::args into Args struct
fn parse() -> Args {
    let mut paths: HashMap<PathBuf, Action> = HashMap::new();

    let mut done_flags = false;
    let mut show = false;
    let mut argv = Vec::new();
    for arg in env::args().skip(1) {
        if done_flags {
            let cstr = CString::new(arg).expect("Bad CString parse");
            argv.push(cstr);
            continue;
        }

        match arg.as_ref() {
            "-l" => show = true,
            "-h" => usage(0),
            "--" => done_flags = true,
            _ => {
                let parts: Vec<&str> = arg.split("=").collect();
                if parts.len() == 0 || parts.len() > 2 {
                    eprintln!("Bad number of parts in: {}", arg);
                    process::exit(1);
                }

                let key = PathBuf::from(&parts[0]);
                if parts.len() == 2 {
                    let replace = PathBuf::from(&parts[1]);
                    paths.insert(key, Action::Replace(replace));
                } else {
                    paths.insert(key, Action::Block(OpenType::All));
                }
            }
        }
    }

    if argv.len() < 1 {
        eprintln!("No program to execute given");
        usage(1);
    }

    Args { paths, show, argv }
}

/// Parse pid address into a path
///
/// This function is marked unsafe as `addr` must be the address of a CString
unsafe fn user_path(pid: Pid, addr: u64) -> PathBuf {
    let mut path: Vec<u8> = Vec::new();
    let mut loc = addr;

    // Read string word by word from child memory address
    'outer: loop {
        let chars = ptrace::read(pid, loc as *mut c_void).expect("Failed to read");
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
    let path = std::str::from_utf8(&path).expect("Failed to parse path");
    PathBuf::from(path)
}

/// Handle an open call.
///
/// Mutates registers and returns false if call is to be blocked
fn handle_open(pid: Pid, syscall: u64, regs: &mut user_regs_struct, args: &Args) -> bool {
    let (name, path_reg, flag_reg) = match syscall {
        SYS_OPEN => ("open", regs.rdi, regs.rsi),
        SYS_OPENAT => ("openat", regs.rsi, regs.rdx),
        SYS_OPEN_BY_HANDLE_AT => ("open_by_handle_at", regs.rsi, regs.r8),
        _ => panic!("Bad syscall passed to handle_open"),
    };

    // Read path from child process
    let path = unsafe { user_path(pid, path_reg) };

    let mut ret = true;
    match args.paths.get(&path) {
        Some(Block) => {
            // Set syscall to invalid value so it fails to open
            regs.orig_rax = -1i64 as u64;
            ptrace::setregs(pid, *regs).expect("Failed to setregs");
            ret = false;
        },
        _ => ()
    }

    if args.show {
        // Print out open call
        eprint!("{}({:?})", name, path);
        if !ret {
            eprint!(" BLOCKED");
        }
        eprintln!();
    }

    ret
}

/// Start child process and begin intercepting its calls to open
fn start(args: Args) {
    // Fork off child program
    let pid = match fork().expect("Failed to fork") {
        ForkResult::Parent { child } => child,
        ForkResult::Child => {
            ptrace::traceme().expect("Failed to traceme");
            if let Err(_) = execvp(&args.argv[0], &args.argv) {
                eprintln!("Failed to execute {:?}", args.argv[0]);
            }
            process::exit(1);
        }
    };

    // Sync with child traceme
    waitpid(pid, None).expect("Failed to waitpid");

    // Kill child if we die
    let mut options = Options::empty();
    options.insert(Options::PTRACE_O_EXITKILL);
    if let Err(_) = ptrace::setoptions(pid, options) {
        eprintln!("Failed to trace child");
        process::exit(1);
    }

    loop {
        // Syscall entrance
        ptrace::syscall(pid).expect("Failed to trace syscall");
        waitpid(pid, None).expect("Failed to waitpid");

        let mut regs = ptrace::getregs(pid).expect("Failed to getregs");
        let syscall = regs.orig_rax;

        let allowed = match syscall {
            // Check if open is allowed
            SYS_OPEN | SYS_OPENAT | SYS_OPEN_BY_HANDLE_AT => {
                handle_open(pid, syscall, &mut regs, &args)
            },
            // Child exited, we will also
            SYS_EXIT | SYS_EXIT_GROUP => {
                process::exit(regs.rdi as i32)
            },
            _ => true,
        };

        // Syscall exit
        ptrace::syscall(pid).expect("Failed to trace syscall");
        waitpid(pid, None).expect("Failed to waitpid");

        // Update return value if blocked
        if !allowed {
            regs.rax = (-(Errno::EPERM as i64)) as u64;
            ptrace::setregs(pid, regs).expect("Failed to setregs");
        }
    }
}

fn main() {
    let args = parse();
    start(args);
}
