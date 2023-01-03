use std::fs;
use std::io::{self, Write, Read};
use std::process::{Command, Stdio};
use std::str;
use serde::Deserialize;
use clap::Parser;

/// Golden test runner
#[derive(Parser, Debug)]
struct Args {
   /// Path to test config
   path: String,
}


#[derive(Debug, Deserialize)]
struct TestConfig {
    test_program: String,
    test_arguments: Option<Vec<String>>,
    test_stdin: Option<String>,
    expected_stdout: Option<String>,
    expected_stderr: Option<String>,
    expected_exit_code: Option<i32>,
}

impl TestConfig {
    fn to_test_case(self) -> Result<TestCase, TestConfigError> {
        let mut assertions = vec![];
        if let Some(stdout) = self.expected_stdout { assertions.push(TestAssertion::AssertStdout(stdout)) }
        if let Some(stderr) = self.expected_stderr { assertions.push(TestAssertion::AssertStderr(stderr)) }
        if let Some(exit_code) = self.expected_exit_code { assertions.push(TestAssertion::AssertExitCode(exit_code)) }

        if assertions.len() == 0 {
            return Err(TestConfigError::ExpectationRequired)
        }

        Ok(TestCase {
            program: self.test_program,
            arguments: self.test_arguments.unwrap_or(vec![]),
            stdin: self.test_stdin,
            assertions,
        })
    }
}

#[derive(Debug)]
enum TestConfigError {
    ExpectationRequired,
}

struct TestCase {
    program: String,
    arguments: Vec<String>,
    stdin: Option<String>,
    assertions: Vec<TestAssertion>,
}

enum TestAssertion {
    AssertStdout(String),
    AssertStderr(String),
    AssertExitCode(i32),
}

struct TestOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[derive(Debug)]
enum TestError {
    FailedToDecodeUtf8,
    MissingExitCode,
    IOError(io::Error),
}


fn main() {
    let args = Args::parse();

    let toml_str = fs::read_to_string(args.path)
        .expect("Should have been able to read the file");
    let test_config: TestConfig = toml::from_str(&toml_str).unwrap();

    let test_case = test_config.to_test_case().unwrap();
    let test_output = run_test_case(&test_case).unwrap();

    let is_success = test_case.assertions.into_iter().all(|assertion| check_assertion(&test_output, &assertion));

    println!("{} - {} '{}' '{}'", if is_success { "ok" } else { "not ok" }, test_output.exit_code, test_output.stdout, test_output.stderr);
}

fn run_test_case(test_case: &TestCase) -> Result<TestOutput, TestError> {
    let mut cmd = Command::new(&test_case.program);
    cmd.args(&test_case.arguments);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(TestError::IOError)?;

    if let Some(stdin_string) = &test_case.stdin {
        let mut stdin = child.stdin.take().expect("Stdin should be configured to pipe");
        stdin.write_all(stdin_string.as_bytes()).map_err(TestError::IOError)?;
    }

    let stdout = read_pipe_to_string(&mut child.stdout.take().expect("Stdout should be configured to pipe"))?;
    let stderr = read_pipe_to_string(&mut child.stderr.take().expect("Stderr should be configured to pipe"))?;

    let exit_status = child.wait().map_err(TestError::IOError)?;
    let exit_code = exit_status.code().map_or(Err(TestError::MissingExitCode), Ok)?;

    Ok(TestOutput {
        stdout,
        stderr,
        exit_code,
    })
}

fn read_pipe_to_string<T>(pipe: &mut T) -> Result<String, TestError> where T: Read {
    let mut buf: Vec<u8> = vec![];
    pipe.read_to_end(&mut buf).map_err(TestError::IOError)?;
    String::from_utf8(buf).map_or(Err(TestError::FailedToDecodeUtf8), Ok)
}

fn check_assertion(output: &TestOutput, assertion: &TestAssertion) -> bool {
    match assertion {
        TestAssertion::AssertStdout(expected_stdout) => &output.stdout == expected_stdout,
        TestAssertion::AssertStderr(expected_stderr) => &output.stderr == expected_stderr,
        TestAssertion::AssertExitCode(expected_exit_code) => &output.exit_code == expected_exit_code,
    }
}
