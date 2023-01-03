use std::fs;
use std::io::{Write, Read};
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


fn main() {
    let args = Args::parse();

    let toml_str = fs::read_to_string(args.path)
        .expect("Should have been able to read the file");
    let test_config: TestConfig = toml::from_str(&toml_str).unwrap();

    let test_case = test_config.to_test_case().unwrap();
    run_test_case(test_case)
}

fn run_test_case(test_case: TestCase) {
    let mut cmd = Command::new(test_case.program);
    cmd.args(test_case.arguments);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();

    if let Some(stdin_string) = test_case.stdin {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(stdin_string.as_bytes()).unwrap();
    }

    let mut stdout = child.stdout.take().unwrap();
    let mut buf: Vec<u8> = vec![];
    stdout.read_to_end(&mut buf).unwrap();

    let stdout_string = String::from_utf8(buf).unwrap();

    let mut stderr = child.stderr.take().unwrap();
    let mut buf: Vec<u8> = vec![];
    stderr.read_to_end(&mut buf).unwrap();

    let stderr_string = String::from_utf8(buf).unwrap();

    let exit_status = child.wait().unwrap();
    let exit_code = exit_status.code().unwrap();

    let is_success = test_case.assertions.into_iter().all(|assertion| { check_assertion(&stdout_string, &stderr_string, &exit_code, &assertion) });

    println!("{} - {} '{}' '{}'", if is_success { "ok" } else { "not ok" }, exit_code, stdout_string, stderr_string);
}

fn check_assertion(stdout: &String, stderr: &String, exit_code: &i32, assertion: &TestAssertion) -> bool {
    match assertion {
        TestAssertion::AssertStdout(expected_stdout) => stdout == expected_stdout,
        TestAssertion::AssertStderr(expected_stderr) => stderr == expected_stderr,
        TestAssertion::AssertExitCode(expected_exit_code) => exit_code == expected_exit_code,
    }
}
