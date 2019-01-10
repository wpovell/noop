use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process;

use crate::err::{Error, Result};

/// Open mode to block
pub enum OpenType {
    Read,
    Write,
    All,
}

/// Action to take for a given file
pub enum Action {
    Block(OpenType),
    Replace(PathBuf),
}

/// Print usage message and exit
pub fn usage(code: i32) -> ! {
    let msg = "noop blocks or modifies calls to open made by the passed program.

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

    eprintln!("{}", msg);
    process::exit(code);
}

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
                Action::Block(_) => writeln!(f, "BLOCK"),
                Action::Replace(p) => writeln!(f, "{:?}", p),
            }?;
        }
        Ok(())
    }
}

/// Parse env::args into Args struct
pub fn parse() -> Result<Args> {
    let mut paths: HashMap<PathBuf, Action> = HashMap::new();

    let mut done_flags = false;
    let mut show = false;
    let mut argv = Vec::new();
    for arg in env::args().skip(1) {
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

                let key = parse_path(&parts[0]);
                if parts.len() == 2 {
                    let replace = PathBuf::from(&parts[1]);
                    paths.insert(key, Action::Replace(replace));
                } else {
                    paths.insert(key, Action::Block(OpenType::All));
                }
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
pub fn parse_path(name: &str) -> PathBuf {
    match fs::canonicalize(name) {
        Ok(full_path) => full_path,
        Err(_) => PathBuf::from(name),
    }
}
