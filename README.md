# noop

Have some program that keeps writing files all over your system? Stop it from opening them with `noop`!

## Usage

```
noop blocks or modifies calls to open made by the passed program.

USAGE:
  noop [-lh] [FILE | FILE=REPLACE] -- PROGRAM [ARGS...]

FLAGS:
  -l Logs open calls and resulting action to stderr
  -h Show this message and exit

ARGS:
  FILE          Block PROGRAM from opening FILE
  FILE=REPLACE  Replace open calls to FILE with REPLACE
  PROGRAM       PROGRAM to run and intercept on
  ARGS          ARGS to pass to the PROGRAM
```

```shell
$ echo foo > bar
$ cat bar
foo
$ noop bar -- cat bar
cat: bar: Operation not permitted
```

## Building

`noop` can be built from source with `cargo build`.

`noop` won't work on anything other than Linux.

## Alternatives

A major downside of this approach is that it intercepts every syscall, including ones that are not `open`. This has a significant performance penalty, similar to running a program under `strace`.

Some alternatives:

- Run the program as an unprivileged user.
- If the program is writing files in your home directory, try exporting `$HOME` as something like `/tmp` before running.
- Provide a custom open using `LD_PRELOAD`. Note that this frequently fails in the case that the function used is statically linked / nonstandard.

## TODO

- Add file open redirection
- Add option to deny opening only for some modes
