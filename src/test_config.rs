use crate::test_case::TestCase;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env::{var, VarError};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn from_str(str: &str) -> Result<TestConfig, ParseTestConfigError> {
    toml::from_str(&str).map_err(|err| ParseTestConfigError { inner: err })
}

pub struct ParseTestConfigError {
    pub inner: toml::de::Error,
}

#[derive(Debug)]
pub enum TestConfigError {
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
        for (id_path, test_config) in test_configs {
            let test_case = test_config.to_test_case(source_file.clone(), id_path, current_dir)?;
            test_cases.push(test_case)
        }

        Ok(test_cases)
    }

    fn to_test_case(
        self,
        source_file: PathBuf,
        id_path: Vec<String>,
        current_dir: &Path,
    ) -> Result<TestCase, TestConfigError> {
        let description = read_from_config_value(self.test_description, current_dir)?;

        let program: String;
        if let Some(p) = read_from_config_value(self.test_program, current_dir)? {
            program = p
        } else {
            return Err(TestConfigError::ProgramRequired);
        }

        let mut arguments = vec![];
        for arg in self.test_arguments.unwrap_or(vec![]) {
            arguments.push(arg.read(current_dir)?)
        }

        let stdin = read_from_config_value(self.test_stdin, current_dir)?;

        let expected_stdout = read_from_config_value(self.expected_stdout, current_dir)?;
        let expected_stderr = read_from_config_value(self.expected_stderr, current_dir)?;
        let expected_exit_code = read_from_config_value(self.expected_exit_code, current_dir)?;

        if expected_stdout == None && expected_stderr == None && expected_exit_code == None {
            return Err(TestConfigError::ExpectationRequired);
        }

        Ok(TestCase {
            source_file,
            id_path,
            description,
            program,
            arguments,
            stdin,
            expected_stdout,
            expected_stderr,
            expected_exit_code,
        })
    }
}

fn read_from_config_value<T>(
    config_value: Option<ConfigValue<T>>,
    current_dir: &Path,
) -> Result<Option<T>, TestConfigError>
where
    T: FromStr,
{
    if let Some(config_value) = config_value {
        Ok(Some(config_value.read(current_dir)?))
    } else {
        Ok(None)
    }
}

// Currently only merges a single level
fn split_test_configs(base_config: TestConfig) -> Vec<(Vec<String>, TestConfig)> {
    if let Some(tests) = base_config.tests.clone() {
        let mut test_configs = vec![];

        for (name, sub_config) in tests.into_iter() {
            let merged_test_config = merge_test_configs(base_config.clone(), sub_config);
            test_configs.push((vec![name], merged_test_config))
        }

        test_configs
    } else {
        vec![(vec![], base_config)]
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
