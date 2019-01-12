# noop

Have some program that keeps writing files all over your system? Stop it from opening them with `noop`!

Note: `noop` won't work on anything other than Linux.

## Usage

```
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
```

## Example

```shell
$ echo foo > bar
$ cat bar
foo
$ # No open
$ noop bar -- cat bar
cat: bar: Operation not permitted
$ # No read
$ noop bar:r -- cat bar
cat: bar: Operation not permitted
$ # No write
$ noop bar:w -- cat bar
foo
$ echo | noop bar:w -- tee bar
tee: bar: Operation not permitted
$ # Redirect
$ noop wrong=bar -- cat wrong
foo
```

## Building

Run `cargo build` to compile.

The project relies on a recently landed PR of the `nix` crate so for now the dependency pulls from GitHub rather than `crates.io`.

## Alternatives

There is some overhead of `noop`, mainly in handling every `open` call.
`noop` uses `seccomp` with to avoid having to handle every syscall, however.

See `cargo bench` for more empirical measure of performance.

Some alternative ways to block the opening of a file:

- Run the program as an unprivileged user.
- If the program is writing files in your home directory, try exporting `$HOME` as something like `/tmp` before running.
- Provide a custom open using `LD_PRELOAD`. Note that this frequently fails in the case that the function used is statically linked / nonstandard.

## Bugs

The command line parsing doesn't work very well on files with `=` or `:` in them.

Applications that fork aren't handled.

## TODO

- Add recursive blocking
- Add folder creation blocking
- Add better command line argument handling
