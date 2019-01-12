#![feature(test)]
extern crate test;
use test::Bencher;

use std::process::Command;

#[cfg(debug_assertions)]
static TARGET: &'static str = "target/debug/noop";

#[cfg(not(debug_assertions))]
static TARGET: &'static str = "target/release/noop";

#[bench]
fn find(b: &mut Bencher) {
    b.iter(|| {
        Command::new("find").output().unwrap();
    });
}

#[bench]
fn noop_find(b: &mut Bencher) {
    b.iter(|| {
        Command::new(TARGET)
            .args(&["--", "find"])
            .output().unwrap();
    });
}

#[bench]
fn cat(b: &mut Bencher) {
    b.iter(|| {
        Command::new("cat")
            .arg("Cargo.toml")
            .output().unwrap();
    });
}

#[bench]
fn noop_cat(b: &mut Bencher) {
    b.iter(|| {
        Command::new(TARGET)
            .args(&["--", "cat", "Cargo.toml"])
            .output().unwrap();
    });
}
