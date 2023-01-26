use crate::file_util;
use crate::test_case::TestCase;
use crate::test_id::TestId;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

// READ TEST CONFIG

pub enum TestConfigResult {
    FailedToReadFile(io::Error),
    FailedToParseTestConfig(toml::de::Error),
    PartialSuccess {
        requirements: TestConfigData,
        error: CreateTestCaseError,
    },
    Success {
        requirements: TestConfigData,
        test_cases: Vec<TestCase>,
    },
}

pub fn test_cases_from_file(path: &Path) -> TestConfigResult {
    let toml_content = match fs::read_to_string(path) {
        Ok(x) => x,
        Err(err) => return TestConfigResult::FailedToReadFile(err),
    };

    let test_config = match toml::from_str::<TestConfig>(&toml_content) {
        Ok(x) => x,
        Err(err) => return TestConfigResult::FailedToParseTestConfig(err),
    };

    let requirements = test_config.get_requirements();
    let test_dir = file_util::parent_dir(path);
    let data = gather_requirements(&requirements, &test_dir);

    let test_cases = match test_config.to_test_cases(path, &data) {
        Ok(x) => x,
        Err(err) => {
            return TestConfigResult::PartialSuccess {
                requirements: data,
                error: err,
            }
        }
    };

    TestConfigResult::Success {
        requirements: data,
        test_cases,
    }
}

// TOML STRUCTURE

#[derive(Deserialize, Clone)]
struct TestConfig {
    test_description: Option<ConfigValue<String>>,
    test_program: Option<ConfigValue<String>>,
    test_arguments: Option<Vec<ConfigValue<String>>>,
    test_stdin: Option<ConfigValue<String>>,
    expected_stdout: Option<ConfigValue<String>>,
    expected_stderr: Option<ConfigValue<String>>,
    expected_exit_code: Option<ConfigValue<i32>>,
    tests: Option<BTreeMap<String, TestConfig>>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum ConfigValue<T> {
    Literal(T),
    WrappedLiteral { value: T },
    ReadFromFile { file: String },
    FetchFromEnv { env: String },
}

// REQUIREMENTS

#[derive(PartialEq, PartialOrd, Eq, Ord)]
enum Requirement {
    LocalFile(String),
    EnvVar(String),
}

impl TestConfig {
    fn get_requirements(&self) -> BTreeSet<Requirement> {
        let mut requirements = BTreeSet::new();

        add_requirement(&mut requirements, &self.test_description);
        add_requirement(&mut requirements, &self.test_program);
        add_requirement(&mut requirements, &self.test_stdin);
        add_requirement(&mut requirements, &self.expected_stdout);
        add_requirement(&mut requirements, &self.expected_stderr);
        add_requirement(&mut requirements, &self.expected_exit_code);

        if let Some(arguments) = &self.test_arguments {
            for argument in arguments {
                let requirement = get_requirement(argument);
                requirements.extend(requirement)
            }
        }

        if let Some(tests) = &self.tests {
            for (_, sub_test_config) in tests {
                let mut sub_requirements = sub_test_config.get_requirements();
                requirements.append(&mut sub_requirements)
            }
        }

        requirements
    }
}

fn add_requirement<T>(requirements: &mut BTreeSet<Requirement>, value: &Option<ConfigValue<T>>) {
    requirements.extend(value.as_ref().and_then(get_requirement));
}

fn get_requirement<T>(config_value: &ConfigValue<T>) -> Option<Requirement> {
    match config_value {
        ConfigValue::Literal(_) => None,
        ConfigValue::WrappedLiteral { value: _ } => None,
        ConfigValue::ReadFromFile { file: filename } => {
            Some(Requirement::LocalFile(filename.clone()))
        }
        ConfigValue::FetchFromEnv { env: var_name } => Some(Requirement::EnvVar(var_name.clone())),
    }
}

// READ CONTENT

pub struct TestConfigData {
    files: BTreeMap<String, Option<String>>,
    env: BTreeMap<String, Option<String>>,
}

impl TestConfigData {
    fn new() -> TestConfigData {
        TestConfigData {
            env: BTreeMap::new(),
            files: BTreeMap::new(),
        }
    }

    fn get_file(&self, key: &String) -> Option<String> {
        self.files.get(key).and_then(|x| x.to_owned())
    }

    fn get_env(&self, key: &String) -> Option<String> {
        self.env.get(key).and_then(|x| x.to_owned())
    }
}

fn gather_requirements(requirements: &BTreeSet<Requirement>, current_dir: &Path) -> TestConfigData {
    let mut data = TestConfigData::new();

    for requirement in requirements {
        match requirement {
            Requirement::LocalFile(path) => {
                data.files
                    .insert(path.to_owned(), read_local_file(path, current_dir).ok());
            }
            Requirement::EnvVar(var_name) => {
                data.env
                    .insert(var_name.to_owned(), read_from_env(var_name).ok());
            }
        }
    }

    data
}

fn read_local_file(path: &String, current_dir: &Path) -> io::Result<String> {
    let path = current_dir.join(path);
    fs::read_to_string(path)
}

fn read_from_env(var_name: &String) -> Result<String, env::VarError> {
    env::var(var_name)
}

// CREATE TEST CASES

pub enum CreateTestCaseError {
    MissingLocalFile(String),
    MissingEnvVar(String),
    FailedToParseString,
    ProgramRequired,
    ExpectationRequired,
}

impl TestConfig {
    fn to_test_cases<P>(
        self,
        path: P,
        data: &TestConfigData,
    ) -> Result<Vec<TestCase>, CreateTestCaseError>
    where
        P: AsRef<Path>,
    {
        let source_file = path.as_ref();

        let test_configs = split_test_configs(self);

        let mut test_cases = vec![];
        for (id_path, test_config) in test_configs {
            let test_id = TestId::new(id_path);
            let test_case = test_config.to_test_case(source_file.to_path_buf(), test_id, data)?;
            test_cases.push(test_case)
        }

        Ok(test_cases)
    }

    fn to_test_case(
        self,
        source_file: PathBuf,
        id: TestId,
        data: &TestConfigData,
    ) -> Result<TestCase, CreateTestCaseError> {
        let description = read_from_config_value(self.test_description, data)?;

        let program: String;
        if let Some(p) = read_from_config_value(self.test_program, data)? {
            program = p
        } else {
            return Err(CreateTestCaseError::ProgramRequired);
        }

        let mut arguments = vec![];
        for arg in self.test_arguments.unwrap_or(vec![]) {
            arguments.push(arg.read(data)?)
        }

        let stdin = read_from_config_value(self.test_stdin, data)?;

        let expected_stdout = read_from_config_value(self.expected_stdout, data)?;
        let expected_stderr = read_from_config_value(self.expected_stderr, data)?;
        let expected_exit_code = read_from_config_value(self.expected_exit_code, data)?;

        if expected_stdout == None && expected_stderr == None && expected_exit_code == None {
            return Err(CreateTestCaseError::ExpectationRequired);
        }

        Ok(TestCase {
            source_file,
            id,
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
    data: &TestConfigData,
) -> Result<Option<T>, CreateTestCaseError>
where
    T: FromStr,
{
    if let Some(config_value) = config_value {
        Ok(Some(config_value.read(data)?))
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

impl<T> ConfigValue<T>
where
    T: FromStr,
{
    fn read(self, data: &TestConfigData) -> Result<T, CreateTestCaseError> {
        match self {
            Self::Literal(value) => Ok(value),
            Self::WrappedLiteral { value } => Ok(value),
            Self::ReadFromFile { file: file_path } => {
                if let Some(str) = data.get_file(&file_path) {
                    let value = str
                        .parse()
                        .map_err(|_err| CreateTestCaseError::FailedToParseString)?;
                    Ok(value)
                } else {
                    Err(CreateTestCaseError::MissingLocalFile(file_path))
                }
            }
            Self::FetchFromEnv { env: var_name } => {
                if let Some(str) = data.get_env(&var_name) {
                    let value = str
                        .parse()
                        .map_err(|_err| CreateTestCaseError::FailedToParseString)?;
                    Ok(value)
                } else {
                    Err(CreateTestCaseError::MissingEnvVar(var_name))
                }
            }
        }
    }
}
