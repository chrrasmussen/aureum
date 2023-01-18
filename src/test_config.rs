use crate::test_case::{TestAssertion, TestCase};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env::{var, VarError};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn from_str(str: &str) -> Result<TestConfig, TestConfigError> {
    toml::from_str(&str).map_err(TestConfigError::InvalidConfig)
}

#[derive(Debug)]
pub enum TestConfigError {
    InvalidConfig(toml::de::Error),
    FailedToFetchEnvVar { var_name: String, error: VarError },
    FailedToParseString(String),
    ProgramRequired,
    ExpectationRequired,
    IOError(io::Error),
}

#[derive(Deserialize, Clone)]
pub struct TestConfig {
    test_description: Option<ConfigValue<String>>,
    test_program: Option<ConfigValue<String>>,
    test_arguments: Option<Vec<ConfigValue<String>>>,
    test_stdin: Option<ConfigValue<String>>,
    expected_stdout: Option<ConfigValue<String>>,
    expected_stderr: Option<ConfigValue<String>>,
    expected_exit_code: Option<ConfigValue<i32>>,
    tests: Option<BTreeMap<String, TestConfig>>,
}

impl TestConfig {
    pub fn to_test_cases<P>(self, path: P) -> Result<Vec<TestCase>, TestConfigError>
    where
        P: Into<PathBuf>,
    {
        let source_file = path.into();
        let current_dir = source_file.as_path().parent().unwrap_or(Path::new("."));

        let test_configs = split_test_configs(self);

        let mut test_cases = vec![];
        for test_config in test_configs {
            let test_case = test_config.to_test_case(source_file.clone(), current_dir)?;
            test_cases.push(test_case)
        }

        Ok(test_cases)
    }

    fn to_test_case(
        self,
        source_file: PathBuf,
        current_dir: &Path,
    ) -> Result<TestCase, TestConfigError> {
        let description: String;
        if let Some(test_name) = self.test_description {
            description = test_name.read(current_dir)?
        } else {
            description = name_from_path(&source_file).unwrap_or(String::from(""))
        }

        let program: String;
        if let Some(test_program) = self.test_program {
            program = test_program.read(current_dir)?
        } else {
            return Err(TestConfigError::ProgramRequired);
        }

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
            description,
            program,
            arguments,
            stdin,
            assertions,
        })
    }
}

// Currently only merges a single level
fn split_test_configs(base_config: TestConfig) -> Vec<TestConfig> {
    if let Some(tests) = base_config.tests.clone() {
        let mut test_configs = vec![];
        for (name, sub_config) in tests.into_iter() {
            let named_sub_config = TestConfig {
                test_description: sub_config
                    .test_description
                    .or(Some(ConfigValue::Literal(name))),
                ..sub_config
            };
            let merged_test_config = merge_test_configs(base_config.clone(), named_sub_config);
            test_configs.push(merged_test_config)
        }
        test_configs
    } else {
        vec![base_config]
    }
}

fn merge_test_configs(base_config: TestConfig, prioritized_config: TestConfig) -> TestConfig {
    TestConfig {
        test_description: prioritized_config
            .test_description
            .or(base_config.test_description),
        test_program: prioritized_config.test_program.or(base_config.test_program),
        test_arguments: prioritized_config
            .test_arguments
            .or(base_config.test_arguments),
        test_stdin: prioritized_config.test_stdin.or(base_config.test_stdin),
        expected_stdout: prioritized_config
            .expected_stdout
            .or(base_config.expected_stdout),
        expected_stderr: prioritized_config
            .expected_stderr
            .or(base_config.expected_stderr),
        expected_exit_code: prioritized_config
            .expected_exit_code
            .or(base_config.expected_exit_code),
        tests: prioritized_config.tests, // Do not propagate tests from `base_config`
    }
}

#[derive(Deserialize, Clone)]
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

fn name_from_path(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy().to_string();

    let name: String;
    if let Some(n) = file_name.strip_suffix(".au.toml") {
        name = String::from(n)
    } else if let Some(n) = file_name.strip_suffix(".toml") {
        name = String::from(n)
    } else {
        name = file_name
    }

    Some(name)
}
