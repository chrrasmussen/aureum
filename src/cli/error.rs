use aureum::formats::tree;
use aureum::formats::tree::Tree::{self, Leaf, Node};
use aureum::test_id::TestId;
use aureum::toml_config::{TestCaseValidationError, TestCases, TomlConfigData};
use relative_path::RelativePathBuf;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

pub struct ConfigError {
    source_file: RelativePathBuf,
    errors: Vec<Tree>,
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let source_file = self.source_file.to_string();
        let content = Node(source_file, self.errors.clone());
        let output =
            tree::draw_tree(&content).unwrap_or_else(|_| String::from("Failed to draw tree\n"));
        write!(f, "{}", output)
    }
}

pub fn report_error(source_file: RelativePathBuf, errors: Vec<Tree>) -> ConfigError {
    ConfigError {
        source_file,
        errors,
    }
}

pub fn report_error_message(source_file: RelativePathBuf, msg: &str) -> ConfigError {
    ConfigError {
        source_file,
        errors: vec![Leaf(vec![String::from(msg)])],
    }
}

pub fn test_cases_errors(test_cases: &TestCases) -> Vec<Tree> {
    let mut categories = vec![];

    let requirements = requirements_map(&test_cases.requirements);
    if !requirements.is_empty() {
        categories.push(Node(String::from("Requirements"), requirements));
    }

    let validation_errors = validation_errors_map(&test_cases.validation_errors);
    if !validation_errors.is_empty() {
        categories.push(Node(String::from("Validation errors"), validation_errors));
    }

    categories
}

fn requirements_map(requirements: &TomlConfigData) -> Vec<Tree> {
    let mut categories = vec![];

    let any_files_missing = requirements.any_missing_file_requirements();
    let files = requirements.file_requirements();
    if any_files_missing && !files.is_empty() {
        categories.push(Node(
            String::from("Files"),
            files
                .into_iter()
                .map(|(x, y)| Leaf(vec![format!("{} {}", show_presence(y), x)]))
                .collect(),
        ));
    }

    let any_env_missing = requirements.any_missing_env_requirements();
    let env = requirements.env_requirements();
    if any_env_missing && !env.is_empty() {
        categories.push(Node(
            String::from("Environment"),
            env.into_iter()
                .map(|(x, y)| Leaf(vec![format!("{} {}", show_presence(y), x)]))
                .collect(),
        ));
    }

    categories
}

fn validation_errors_map(
    validation_errors: &Vec<(TestId, BTreeSet<TestCaseValidationError>)>,
) -> Vec<Tree> {
    if validation_errors.len() == 1 {
        let (maybe_root, errs) = &validation_errors[0];
        if maybe_root.is_root() {
            return errs
                .iter()
                .map(|err| Leaf(vec![show_validation_error(err)]))
                .collect::<Vec<_>>();
        }
    }

    let mut test_cases = vec![];

    for (test_id, errs) in validation_errors {
        test_cases.push(Node(
            test_id.to_string(),
            errs.iter()
                .map(|err| Leaf(vec![show_validation_error(err)]))
                .collect(),
        ));
    }

    test_cases
}

fn show_validation_error(validation_error: &TestCaseValidationError) -> String {
    match validation_error {
        TestCaseValidationError::MissingExternalFile(file_path) => {
            format!("Missing external file '{}'", file_path)
        }
        TestCaseValidationError::MissingEnvVar(var_name) => {
            format!("Missing environment variable '{}'", var_name)
        }
        TestCaseValidationError::FailedToParseString => String::from("Failed to parse string"),
        TestCaseValidationError::ProgramRequired => String::from("The field 'program' is required"),
        TestCaseValidationError::ProgramNotFound(program) => {
            format!("The program '{}' was not found", program)
        }
        TestCaseValidationError::ExpectationRequired => {
            String::from("At least one expectation is required")
        }
    }
}

fn show_presence(value: bool) -> String {
    String::from(if value { "✔️" } else { "❌" })
}
