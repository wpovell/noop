extern crate rand;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use std::io::Write;
use std::process;
use std::{env, fs, panic};

#[cfg(debug_assertions)]
static TARGET: &'static str = "target/debug/noop";

#[cfg(not(debug_assertions))]
static TARGET: &'static str = "target/release/noop";

pub static TEST: &'static str = "DEADBEEF";

/// Run test, passing it a temp file that is cleaned up after
///
/// Created file has the string "test" in it.
pub fn with_tempfile<T>(test: T) -> ()
where
    T: FnOnce(&str) -> () + panic::UnwindSafe,
{
    // Generate random name
    let name: String = thread_rng().sample_iter(&Alphanumeric).take(30).collect();
    let mut file = env::temp_dir();
    file.push(name);
    let file = file.to_str().unwrap();

    // Create
    let mut f = fs::File::create(file).unwrap();
    f.write(TEST.as_bytes()).unwrap();
    drop(f);

    // Test
    let result = panic::catch_unwind(|| test(&file));

    // Delete
    let _ = fs::remove_file(file);

    assert!(result.is_ok())
}

/// Wrapper around process::Output that requires less unwrapping
pub struct Output {
    pub out: String,
    pub err: String,
    pub status: i32,
}

impl Output {
    /// Create new output struct from `process::Output`
    fn new(out: process::Output) -> Output {
        Output {
            out: String::from_utf8(out.stdout).unwrap(),
            err: String::from_utf8(out.stderr).unwrap(),
            status: out.status.code().unwrap(),
        }
    }

    /// Returns true if command failed
    pub fn fail(&self) -> bool {
        self.status != 0
    }

    /// Returns true if command succeded
    pub fn pass(&self) -> bool {
        self.status == 0
    }

    /// Returns true if `stdout` or `stderr` contains `s`
    pub fn contains(&self, s: &str) -> bool {
        self.out.contains(s) || self.err.contains(s)
    }
}

/// Returns output wrapper for `TARGET` run with `args`
pub fn output(args: &[&str]) -> Output {
    Output::new(process::Command::new(TARGET).args(args).output().unwrap())
}
