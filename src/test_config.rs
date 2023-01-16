use crate::test_case::{TestAssertion, TestCase};
use serde::Deserialize;
use std::env::{var, VarError};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Deserialize)]
pub struct TestConfig {
    test_program: ConfigValue<String>,
    test_arguments: Option<Vec<ConfigValue<String>>>,
    test_stdin: Option<ConfigValue<String>>,
    expected_stdout: Option<ConfigValue<String>>,
    expected_stderr: Option<ConfigValue<String>>,
    expected_exit_code: Option<ConfigValue<i32>>,
}

impl TestConfig {
    pub fn to_test_case<P>(self, path: P) -> Result<TestCase, TestConfigError>
    where
        P: Into<PathBuf>,
    {
        let source_file = path.into();
        let current_dir = source_file.as_path().parent().unwrap_or(Path::new("."));

        let program = self.test_program.read(current_dir)?;

        let mut arguments = vec![];
        for arg in self.test_arguments.unwrap_or(vec![]) {
            arguments.push(arg.read(current_dir)?)
        }

        let stdin: Option<String>;
        if let Some(test_stdin) = self.test_stdin {
            stdin = Some(test_stdin.read(current_dir)?)
        } else {
            stdin = None
        }

        let mut assertions = vec![];
        if let Some(stdout) = self.expected_stdout {
            assertions.push(TestAssertion::AssertStdout(stdout.read(current_dir)?))
        }
        if let Some(stderr) = self.expected_stderr {
            assertions.push(TestAssertion::AssertStderr(stderr.read(current_dir)?))
        }
        if let Some(exit_code) = self.expected_exit_code {
            assertions.push(TestAssertion::AssertExitCode(exit_code.read(current_dir)?))
        }

        if assertions.len() == 0 {
            return Err(TestConfigError::ExpectationRequired);
        }

        Ok(TestCase {
            source_file,
            program,
            arguments,
            stdin,
            assertions,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum TestConfigError {
    FailedToFetchEnvVar { var_name: String, error: VarError },
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
    FetchFromEnv { env: String },
}

impl<T> ConfigValue<T>
where
    T: FromStr,
{
    fn read(self, current_dir: &Path) -> Result<T, TestConfigError> {
        match self {
            Self::Literal(value) => Ok(value),
            Self::WrappedLiteral { value } => Ok(value),
            Self::ReadFromFile { file } => {
                let path = current_dir.join(file);
                let str = fs::read_to_string(path).map_err(TestConfigError::IOError)?;
                let value = str
                    .parse()
                    .map_err(|_err| TestConfigError::FailedToParseString(str))?;
                Ok(value)
            }
            Self::FetchFromEnv { env } => {
                let str = var(&env).map_err(|err| TestConfigError::FailedToFetchEnvVar {
                    var_name: env,
                    error: err,
                })?;
                let value = str
                    .parse()
                    .map_err(|_err| TestConfigError::FailedToParseString(str))?;
                Ok(value)
            }
        }
    }
}