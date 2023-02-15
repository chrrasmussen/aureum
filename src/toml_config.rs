use crate::test_case::TestCase;
use crate::test_id::TestId;
use crate::utils::file;
use relative_path::{RelativePath, RelativePathBuf};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

// READ CONFIG FILE

pub struct TestCases {
    pub requirements: TomlConfigData,
    pub validation_errors: Vec<(TestId, BTreeSet<TestCaseValidationError>)>,
    pub test_cases: Vec<TestCase>,
}

pub enum TomlConfigError {
    FailedToReadFile(io::Error),
    FailedToParseTomlConfig(toml::de::Error),
}

pub fn test_cases_from_file(source_file: &RelativePath) -> Result<TestCases, TomlConfigError> {
    let source_path = source_file.to_logical_path(".");

    let toml_content =
        fs::read_to_string(source_path).map_err(TomlConfigError::FailedToReadFile)?;
    let toml_config = toml::from_str::<TomlConfig>(&toml_content)
        .map_err(TomlConfigError::FailedToParseTomlConfig)?;

    let requirements = toml_config.get_requirements();
    let source_dir = file::parent_dir(source_file).to_logical_path(".");
    let data = gather_requirements(&requirements, &source_dir);

    let (test_cases, validation_errors) = toml_config.to_test_cases(source_file, &data);

    Ok(TestCases {
        requirements: data,
        validation_errors,
        test_cases,
    })
}

// TOML STRUCTURE

#[derive(Deserialize, Clone)]
struct TomlConfig {
    description: Option<ConfigValue<String>>,
    program: Option<ConfigValue<String>>,
    program_arguments: Option<Vec<ConfigValue<String>>>,
    stdin: Option<ConfigValue<String>>,
    expected_stdout: Option<ConfigValue<String>>,
    expected_stderr: Option<ConfigValue<String>>,
    expected_exit_code: Option<ConfigValue<i32>>,
    tests: Option<BTreeMap<String, TomlConfig>>,
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

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Requirement {
    ExternalFile(String),
    EnvVar(String),
}

impl TomlConfig {
    fn get_requirements(&self) -> BTreeSet<Requirement> {
        let mut requirements = BTreeSet::new();

        add_requirement(&mut requirements, &self.description);
        add_requirement(&mut requirements, &self.program);
        add_requirement(&mut requirements, &self.stdin);
        add_requirement(&mut requirements, &self.expected_stdout);
        add_requirement(&mut requirements, &self.expected_stderr);
        add_requirement(&mut requirements, &self.expected_exit_code);

        if let Some(arguments) = &self.program_arguments {
            for argument in arguments {
                let requirement = get_requirement(argument);
                requirements.extend(requirement)
            }
        }

        if let Some(tests) = &self.tests {
            for (_, sub_toml_config) in tests {
                let mut sub_requirements = sub_toml_config.get_requirements();
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
            Some(Requirement::ExternalFile(filename.clone()))
        }
        ConfigValue::FetchFromEnv { env: var_name } => Some(Requirement::EnvVar(var_name.clone())),
    }
}

// READ CONTENT

pub struct TomlConfigData {
    files: BTreeMap<String, Option<String>>,
    env: BTreeMap<String, Option<String>>,
}

impl TomlConfigData {
    fn new() -> TomlConfigData {
        TomlConfigData {
            env: BTreeMap::new(),
            files: BTreeMap::new(),
        }
    }

    pub fn any_missing_file_requirements(&self) -> bool {
        self.files.iter().any(|(_key, value)| value.is_none())
    }

    pub fn file_requirements(&self) -> Vec<(String, bool)> {
        Vec::from_iter(
            self.files
                .iter()
                .map(|(key, value)| (key.to_owned(), value.is_some())),
        )
    }

    pub fn any_missing_env_requirements(&self) -> bool {
        self.env.iter().any(|(_key, value)| value.is_none())
    }

    pub fn env_requirements(&self) -> Vec<(String, bool)> {
        Vec::from_iter(
            self.env
                .iter()
                .map(|(key, value)| (key.to_owned(), value.is_some())),
        )
    }

    fn get_file(&self, key: &String) -> Option<String> {
        self.files.get(key).and_then(|x| x.to_owned())
    }

    fn get_env(&self, key: &String) -> Option<String> {
        self.env.get(key).and_then(|x| x.to_owned())
    }
}

fn gather_requirements(requirements: &BTreeSet<Requirement>, current_dir: &Path) -> TomlConfigData {
    let mut data = TomlConfigData::new();

    for requirement in requirements {
        match requirement {
            Requirement::ExternalFile(path) => {
                data.files
                    .insert(path.to_owned(), read_external_file(path, current_dir).ok());
            }
            Requirement::EnvVar(var_name) => {
                data.env
                    .insert(var_name.to_owned(), read_from_env(var_name).ok());
            }
        }
    }

    data
}

fn read_external_file(path: &String, current_dir: &Path) -> io::Result<String> {
    let path = current_dir.join(path);
    fs::read_to_string(path)
}

fn read_from_env(var_name: &String) -> Result<String, env::VarError> {
    env::var(var_name)
}

// CREATE TEST CASES

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum TestCaseValidationError {
    MissingExternalFile(String),
    MissingEnvVar(String),
    FailedToParseString,
    ProgramRequired,
    ProgramNotFound(String),
    ExpectationRequired,
}

impl TomlConfig {
    fn to_test_cases<P>(
        self,
        path: P,
        data: &TomlConfigData,
    ) -> (
        Vec<TestCase>,
        Vec<(TestId, BTreeSet<TestCaseValidationError>)>,
    )
    where
        P: AsRef<RelativePath>,
    {
        let source_file = path.as_ref();

        let toml_configs = split_toml_configs(self);

        let mut test_cases = vec![];
        let mut validation_errors = vec![];

        for (id_path, toml_config) in toml_configs {
            let test_id = TestId::new(id_path);

            match toml_config.to_test_case(source_file.to_owned(), test_id.clone(), data) {
                Ok(test_case) => test_cases.push(test_case),
                Err(err) => validation_errors.push((test_id, err)),
            }
        }

        (test_cases, validation_errors)
    }

    fn to_test_case(
        self,
        source_file: RelativePathBuf,
        id: TestId,
        data: &TomlConfigData,
    ) -> Result<TestCase, BTreeSet<TestCaseValidationError>> {
        let current_dir = file::parent_dir(&source_file);
        let mut validation_errors = BTreeSet::new();

        // Validate fields in config file

        if self.program.is_none() {
            validation_errors.insert(TestCaseValidationError::ProgramRequired);
        }

        if self.expected_stdout.is_none()
            && self.expected_stderr.is_none()
            && self.expected_exit_code.is_none()
        {
            validation_errors.insert(TestCaseValidationError::ExpectationRequired);
        }

        // Read fields

        let description = read_from_config_value(&mut validation_errors, self.description, data);

        let mut program_name = String::from("");
        if let Some(p) = read_from_config_value(&mut validation_errors, self.program, data) {
            program_name = p;
        }

        let mut arguments = vec![];
        for arg in self.program_arguments.unwrap_or(vec![]) {
            match arg.read(data) {
                Ok(arg) => {
                    arguments.push(arg);
                }
                Err(err) => {
                    validation_errors.insert(err);
                }
            }
        }

        let stdin = read_from_config_value(&mut validation_errors, self.stdin, data);

        let expected_stdout =
            read_from_config_value(&mut validation_errors, self.expected_stdout, data);
        let expected_stderr =
            read_from_config_value(&mut validation_errors, self.expected_stderr, data);
        let expected_exit_code =
            read_from_config_value(&mut validation_errors, self.expected_exit_code, data);

        // Validate fields
        let mut program = PathBuf::new();
        if &program_name.is_empty() == &false {
            if let Ok(p) =
                file::find_executable_path(&program_name, current_dir.to_logical_path("."))
            {
                program = p;
            } else {
                validation_errors.insert(TestCaseValidationError::ProgramNotFound(
                    program_name.clone(),
                ));
            }
        }

        if validation_errors.is_empty() {
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
        } else {
            Err(validation_errors)
        }
    }
}

fn read_from_config_value<T>(
    validation_errors: &mut BTreeSet<TestCaseValidationError>,
    config_value: Option<ConfigValue<T>>,
    data: &TomlConfigData,
) -> Option<T>
where
    T: FromStr,
{
    if let Some(config_value) = config_value {
        match config_value.read(data) {
            Ok(value) => Some(value),
            Err(err) => {
                validation_errors.insert(err);
                None
            }
        }
    } else {
        None
    }
}

// Currently only merges a single level
fn split_toml_configs(base_config: TomlConfig) -> Vec<(Vec<String>, TomlConfig)> {
    if let Some(tests) = base_config.tests.clone() {
        let mut toml_configs = vec![];

        for (name, sub_config) in tests.into_iter() {
            let merged_toml_config = merge_toml_configs(base_config.clone(), sub_config);
            toml_configs.push((vec![name], merged_toml_config))
        }

        toml_configs
    } else {
        vec![(vec![], base_config)]
    }
}

fn merge_toml_configs(base_config: TomlConfig, prioritized_config: TomlConfig) -> TomlConfig {
    TomlConfig {
        description: prioritized_config.description.or(base_config.description),
        program: prioritized_config.program.or(base_config.program),
        program_arguments: prioritized_config
            .program_arguments
            .or(base_config.program_arguments),
        stdin: prioritized_config.stdin.or(base_config.stdin),
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
    fn read(self, data: &TomlConfigData) -> Result<T, TestCaseValidationError> {
        match self {
            Self::Literal(value) => Ok(value),
            Self::WrappedLiteral { value } => Ok(value),
            Self::ReadFromFile { file: file_path } => {
                if let Some(str) = data.get_file(&file_path) {
                    let value = str
                        .parse()
                        .map_err(|_err| TestCaseValidationError::FailedToParseString)?;
                    Ok(value)
                } else {
                    Err(TestCaseValidationError::MissingExternalFile(file_path))
                }
            }
            Self::FetchFromEnv { env: var_name } => {
                if let Some(str) = data.get_env(&var_name) {
                    let value = str
                        .parse()
                        .map_err(|_err| TestCaseValidationError::FailedToParseString)?;
                    Ok(value)
                } else {
                    Err(TestCaseValidationError::MissingEnvVar(var_name))
                }
            }
        }
    }
}