//! Command line argument parsing

use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process;

use crate::err::{Error, Result};
use crate::types::{Action, OpenType};

/// Wrapper for arugments passed to program
pub struct Args {
    pub paths: HashMap<PathBuf, Action>,
    pub show: bool,
    pub argv: Vec<CString>,
}

impl fmt::Debug for Args {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "show: {}", self.show)?;
        writeln!(f, "args: {:?}", self.argv)?;
        writeln!(f, "paths:")?;
        for (path, action) in &self.paths {
            write!(f, "\t{:?} => ", path)?;
            match action {
                Action::Block(mode) => writeln!(f, "Block {}", mode),
                Action::Replace(p) => writeln!(f, "{:?}", p),
            }?;
        }
        Ok(())
    }
}

/// Usage message shown by `-h`
static USAGE: &'static str = "\
noop blocks or modifies calls to open made by the passed program.

USAGE:
  noop [-lh] [FILE[:rw] | FILE=REPLACE]... -- PROGRAM [ARG]...

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

/// Print usage message and exit
pub fn usage(code: i32) -> ! {
    eprintln!("{}", USAGE);
    process::exit(code);
}

/// Parse `env::args` into `Args` struct
pub fn parse(args: env::Args) -> Result<Args> {
    let mut paths: HashMap<PathBuf, Action> = HashMap::new();

    let mut done_flags = false;
    let mut show = false;
    let mut argv = Vec::new();
    for arg in args.skip(1) {
        if done_flags {
            let cstr = CString::new(arg)?;
            argv.push(cstr);
            continue;
        }

        match arg.as_ref() {
            "-l" => show = true,
            "-h" => usage(0),
            "--" => done_flags = true,
            _ => {
                let parts: Vec<&str> = arg.split('=').collect();
                if parts.is_empty() || parts.len() > 2 {
                    return Err(Error::Arg {
                        reason: "Bad number of parts in path \"{}\"",
                    });
                }

                let first = parts[0];
                let (path, action) = if parts.len() == 2 {
                    // Replace
                    let replace = PathBuf::from(&parts[1]);
                    (first, Action::Replace(replace))
                } else if first.ends_with(":w") {
                    // No write
                    let new = first.get(..first.len() - 2).unwrap();
                    (new, Action::Block(OpenType::Write))
                } else if first.ends_with(":r") {
                    // No read
                    let new = first.get(..first.len() - 2).unwrap();
                    (new, Action::Block(OpenType::Read))
                } else {
                    // No open
                    (first, Action::Block(OpenType::All))
                };

                let key = parse_path(&path);
                paths.insert(key, action);
            }
        }
    }

    if argv.is_empty() {
        Err(Error::Arg {
            reason: "No program to execute given",
        })
    } else {
        Ok(Args { paths, show, argv })
    }
}

/// Parse name into canonicalize path if possible
///
/// Returns path unchanged if does not exist
pub fn parse_path(name: &str) -> PathBuf {
    match fs::canonicalize(name) {
        Ok(full_path) => full_path,
        Err(_) => PathBuf::from(name),
    }
}
