# Aureum - Golden test runner for executables

**🚧 Work in progress 🚧**

Inspired by [Idris 2's golden test runner](https://github.com/idris-lang/Idris2/tree/main/tests).

Key functionality:
- Language-agnostic: Configure tests using [TOML](http://toml.io) files
- A configuration file may contain multiple tests
- Each test can provide the expected value for `stdout`, `stderr` and `exit code` (See [format](#aureum-configuration-format) below)
- Tests are allowed to reference environment variables and external files
- Supports two output formats: `summary` and [`tap`](http://testanything.org)
- Tries to provide helpful error messages
- `aureum` is tested by `aureum` (See [`spec/`](spec/))
- Runs on Linux, macOS and Windows

This tool is best suited to test executables that are stateless, i.e. running an executable with a given input always produces the same output.


## Installation

1. `git clone http://github.com/chrrasmussen/aureum`
2. `cd aureum`
3. `cargo build` (or `cargo build --release` to build a release version)

The `aureum` executable is now available in the `target/debug` directory.


## Basic usage

```bash
aureum [OPTIONS] <PATHS>...
```

Detailed usage is shown below:

```bash
$ aureum --help
Golden test runner for executables

Usage: aureum [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...  Paths to config files

Options:
      --output-format <OUTPUT_FORMAT>  Options: summary, tap [default: summary]
      --show-all-tests                 Show all tests in summary, regardless of test status
      --run-tests-in-parallel          Run tests in parallel
  -h, --help                           Print help information
```

When running `aureum`, you may specify one or more files/directories/[glob patterns](https://en.wikipedia.org/wiki/Glob_(programming)). When specifying a directory, `aureum` will search for files with the file extension `.au.toml`. This file extension was chosen to allow unrelated `.toml` files to be located in the same directory structure as the Aureum-specific config files.


## Example

Create a file named `hello.au.toml` with the following contents:

```toml
program = "echo"
program_arguments = ["-n", "Hello world"]

expected_stdout = "Hello world"
```

Running the command `aureum hello.au.toml` will output the following:

```bash
Running 1 tests:
.

Test result: OK (1 passed, 0 failed)
```


## Aureum configuration format

The following fields are supported in an Aureum config file:

```toml
description = <string>
program = <string> # Required
program_arguments = <list of strings>
stdin = <string>

# At least one of the following fields are required
expected_stdout = <string>
expected_stderr = <string>
expected_exit_code = <integer>
```

In addition to the literal values mentioned above, the following special forms are available:
- `{ env = "MY_ENV_VAR" }` — Read the value from the environment variable named `MY_ENV_VAR`.
- `{ file = "my_test.stdout" }` — Read the external file `my_test.stdout` from the same directory as the config file.

Recommended file extension: `.au.toml`


### Multiple tests per file

An Aureum config file may contain multiple tests. To specify a sub-test you can add a heading using the following format: `[tests.<name-of-test>]` and configure a test as normal.

Note that fields specified at the level above the sub-test will get inherited by the sub-tests. Because of this, only the leaf nodes are considered a test. The following example configures two tests, where both tests run the program `/bin/echo`:

Filename: ``multiple-tests.au.toml``

```toml
program = "echo"

[tests.test1]
program_arguments = ["-n", "Test 1"]
expected_stdout = "Test 1"

[tests.test2]
program_arguments = ["-n", "Test 2"]
expected_stdout = "Test 2"
```

Running the command `aureum multiple-tests.au.toml` will output the following:

```bash
Running 2 tests:
..

Test result: OK (2 passed, 0 failed)
```


## Alternative tools

- [trycmd](https://github.com/assert-rs/trycmd) [Rust]
- [Golden Tests](https://github.com/jfecher/golden-tests) [Rust]
- [Smoke](https://github.com/SamirTalwar/smoke) [Haskell]
- [goldplate](https://github.com/fugue/goldplate) [Haskell]
- [REPLica](https://github.com/ReplicaTest/REPLica) [Idris]


## License

Aureum is released under the [3-clause BSD license](LICENSE).
