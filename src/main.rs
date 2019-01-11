//! Main entrypoint to binary
#![feature(try_trait)]

use std::env;
use std::process;

mod args;
mod child;
mod err;
mod intercept;
mod types;

fn main() {
    match args::parse(env::args()) {
        Err(e) => {
            eprintln!("Malformed arguments: {}\n", e);
            args::usage(1);
        }
        Ok(args) => {
            if let Err(e) = intercept::start(&args) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    }
}
