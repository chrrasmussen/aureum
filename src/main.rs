mod test_case;

use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::io;
use std::str::FromStr;
use test_case::{TestAssertion, TestCase, TestOutput};

/// Golden test runner
#[derive(Parser)]
struct Args {
    /// Path to test config
    path: String,
}

#[derive(Deserialize)]
struct TestConfig {
    test_program: ConfigValue<String>,
    test_arguments: Option<Vec<ConfigValue<String>>>,
    test_stdin: Option<ConfigValue<String>>,
    expected_stdout: Option<ConfigValue<String>>,
    expected_stderr: Option<ConfigValue<String>>,
    expected_exit_code: Option<ConfigValue<i32>>,
}

impl TestConfig {
    fn to_test_case(self) -> Result<TestCase, TestConfigError> {
        let program = self.test_program.read()?;

        let mut arguments = vec![];
        for arg in self.test_arguments.unwrap_or(vec![]) {
            arguments.push(arg.read()?)
        }

        let stdin: Option<String>;
        if let Some(test_stdin) = self.test_stdin {
            stdin = Some(test_stdin.read()?)
        } else {
            stdin = None
        }

        let mut assertions = vec![];
        if let Some(stdout) = self.expected_stdout {
            assertions.push(TestAssertion::AssertStdout(stdout.read()?))
        }
        if let Some(stderr) = self.expected_stderr {
            assertions.push(TestAssertion::AssertStderr(stderr.read()?))
        }
        if let Some(exit_code) = self.expected_exit_code {
            assertions.push(TestAssertion::AssertExitCode(exit_code.read()?))
        }

        if assertions.len() == 0 {
            return Err(TestConfigError::ExpectationRequired);
        }

        Ok(TestCase {
            program,
            arguments,
            stdin,
            assertions,
        })
    }
}

#[derive(Debug)]
enum TestConfigError {
    FailedToParseString(String),
    ExpectationRequired,
    IOError(io::Error),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ConfigValue<T> {
    Literal(T),
    WrappedLiteral { value: T },
    ReadFromFile { file: String },
}

impl<T> ConfigValue<T>
where
    T: FromStr,
{
    fn read(self) -> Result<T, TestConfigError> {
        match self {
            Self::Literal(value) => Ok(value),
            Self::WrappedLiteral { value } => Ok(value),
            Self::ReadFromFile { file } => {
                let str = fs::read_to_string(file).map_err(TestConfigError::IOError)?;
                let value = str
                    .parse()
                    .map_err(|_err| TestConfigError::FailedToParseString(str))?;
                Ok(value)
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    let toml_str = fs::read_to_string(args.path).expect("Should have been able to read the file");
    let test_config: TestConfig = toml::from_str(&toml_str).unwrap();

    let test_case = test_config.to_test_case().unwrap();
    let test_output = test_case::run(&test_case).unwrap();

    let is_success = test_case
        .assertions
        .into_iter()
        .all(|assertion| check_assertion(&test_output, &assertion));

    println!(
        "{} - {} '{}' '{}'",
        if is_success { "ok" } else { "not ok" },
        test_output.exit_code,
        test_output.stdout,
        test_output.stderr
    );
}

fn check_assertion(output: &TestOutput, assertion: &TestAssertion) -> bool {
    match assertion {
        TestAssertion::AssertStdout(expected_stdout) => &output.stdout == expected_stdout,
        TestAssertion::AssertStderr(expected_stderr) => &output.stderr == expected_stderr,
        TestAssertion::AssertExitCode(expected_exit_code) => {
            &output.exit_code == expected_exit_code
        }
    }
}
