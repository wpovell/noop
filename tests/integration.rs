mod util;
use crate::util::*;

/// Test that no output fails
#[test]
fn no_args() {
    let o = output(&[]);
    assert!(o.fail());
}

/// Test that no program fails
#[test]
fn no_program() {
    let o = output(&["a", "b", "-l", "--"]);
    assert!(o.fail());
}

/// Test that help flag works
#[test]
fn help() {
    let o = output(&["-h"]);
    assert!(o.pass());
    assert!(o.contains("USAGE"));
}

/// Test that write block works
#[test]
fn no_write() {
    with_tempfile(|f| {
        let block = &format!("{}:w", f);
        let o = output(&[block, "--", "cat", f]);
        assert!(o.pass());
        assert!(o.contains(TEST));

        let o = output(&[block, "--", "tee", f]);
        assert!(o.fail());
    });
}

/// Test that read block works
#[test]
fn no_read() {
    with_tempfile(|f| {
        let block = &format!("{}:r", f);
        let o = output(&[block, "--", "cat", f]);
        assert!(o.fail());
        assert!(!o.contains(TEST));

        let o = output(&[block, "--", "tee", f]);
        assert!(o.pass());
    });
}

/// Test that open block works
#[test]
fn no_open() {
    with_tempfile(|f| {
        let o = output(&[f, "--", "cat", f]);
        assert!(o.fail());
        assert!(!o.contains(TEST));

        let o = output(&[f, "--", "tee", f]);
        assert!(o.fail());
    });
}

/// Test that logging works
#[test]
fn log() {
    with_tempfile(|f| {
        let o = output(&["-l", "--", "cat", f]);
        let s = format!("openat(\"{}\", R)", f);
        assert!(o.pass());
        assert!(o.contains(&s));
        assert!(o.contains(TEST));

        let o = output(&["-l", f, "--", "cat", f]);
        let s = format!("{} BLOCKED", s);
        assert!(o.fail());
        assert!(o.contains(&s));
        assert!(!o.contains(TEST));
    });
}

/// Test that path redirection works
#[test]
fn redirect() {
    with_tempfile(|f| {
        let o = output(&["--", "cat", "bar"]);
        assert!(o.fail());
        assert!(!o.contains(TEST));

        let r = &format!("bar={}", f);
        let o = output(&[r, "--", "cat", "bar"]);
        assert!(o.pass());
        assert!(o.contains(TEST));
    });
}
