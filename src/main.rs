mod cli;
mod file_util;
mod tap_format;
mod test_case;
mod test_config;
mod test_id;
mod test_id_container;
mod test_runner;

use cli::{Args, OutputFormat, TestPath};
use glob::glob;
use pathdiff;
use relative_path::RelativePathBuf;
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::exit;
use test_config::{TestCaseValidationError, TestCases, TestConfigData, TestConfigError};
use test_id::TestId;
use test_id_container::TestIdContainer;
use test_runner::{ReportConfig, ReportFormat};

const TEST_FAILURE_EXIT_CODE: i32 = 1;
const INVALID_USER_INPUT_EXIT_CODE: i32 = 2;

fn main() {
    let args = cli::parse();

    let current_dir = env::current_dir().expect("Current directory must be available");

    let source_files = expand_test_paths(&args.paths, &current_dir)
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    if source_files.is_empty() {
        eprintln!("error: No config files found for the given paths");
        exit(INVALID_USER_INPUT_EXIT_CODE);
    }

    let mut all_test_cases = vec![];
    let mut failed_configs = vec![];

    for source_file in source_files {
        match test_config::test_cases_from_file(&source_file) {
            Ok(result) => {
                if let Some(data) = test_cases_errors(&result) {
                    failed_configs.push(report_error(source_file, data));
                }

                all_test_cases.extend(result.test_cases);
            }
            Err(err) => {
                let msg = match err {
                    TestConfigError::FailedToReadFile(_) => "Failed to read file",
                    TestConfigError::FailedToParseTestConfig(_) => "Failed to parse config file",
                };
                failed_configs.push(report_error_message(source_file, msg));
            }
        }
    }

    for failed_config in &failed_configs {
        eprint!("{}", failed_config); // Already contains newline
        eprintln!();
    }

    let report_config = ReportConfig {
        number_of_tests: all_test_cases.len(),
        format: get_report_format(&args),
    };

    let run_results =
        test_runner::run_test_cases(&report_config, &all_test_cases, args.run_tests_in_parallel);

    let any_failed_configs = failed_configs.is_empty() == false;
    if any_failed_configs {
        eprintln!("Some config files contain errors (See above)");
    }

    let all_tests_passed = run_results
        .iter()
        .fold(true, |acc, t| acc && t.is_success());

    if any_failed_configs || all_tests_passed == false {
        exit(TEST_FAILURE_EXIT_CODE)
    }
}

pub fn expand_test_paths(
    test_paths: &[TestPath],
    current_dir: &Path,
) -> BTreeMap<RelativePathBuf, TestIdContainer> {
    let mut files = BTreeMap::new();

    for test_path in test_paths {
        match test_path {
            TestPath::Pipe => {} // Skip
            TestPath::Glob(path) => {
                // TODO: Handle error case
                if let Ok(found_test_files) = locate_test_files(path.as_str()) {
                    for found_test_file in found_test_files {
                        if let Some(path) = get_relative_path(&found_test_file, current_dir) {
                            files.insert(path, TestIdContainer::full());
                        } else {
                            // TODO: Handle if path is not relative
                        }
                    }
                }
            }
            TestPath::SpecificFile {
                source_file,
                test_id,
            } => {
                if let Some(path) = get_relative_path(source_file, current_dir) {
                    files
                        .entry(path)
                        .and_modify(|test_ids: &mut TestIdContainer| {
                            test_ids.add(test_id.clone());
                        })
                        .or_insert(TestIdContainer::empty());
                } else {
                    // TODO: Handle if path is not relative
                }
            }
        }
    }

    files
}

enum LocateFileError {
    InvalidPattern(glob::PatternError),
    InvalidEntry(glob::GlobError),
}

fn locate_test_files(path: &str) -> Result<Vec<PathBuf>, LocateFileError> {
    let mut output = vec![];

    let entries = glob(path).map_err(LocateFileError::InvalidPattern)?;
    for entry in entries {
        let e = entry.map_err(LocateFileError::InvalidEntry)?;
        if e.is_file() {
            output.push(e);
        } else if e.is_dir() {
            // Look for `.au.toml` files in directory (recursively)
            if let Some(search_path) = e.join("**/*.au.toml").to_str() {
                let found_test_files = locate_test_files(search_path)?;
                output.extend(found_test_files);
            }
        }
    }

    Ok(output)
}

fn get_report_format(args: &Args) -> ReportFormat {
    match args.output_format {
        OutputFormat::Summary => ReportFormat::Summary {
            show_all_tests: args.show_all_tests,
        },
        OutputFormat::Tap => ReportFormat::Tap,
    }
}

fn get_relative_path(path: &Path, base: &Path) -> Option<RelativePathBuf> {
    if path.is_relative() {
        RelativePathBuf::from_path(path).ok()
    } else {
        let path_diff = pathdiff::diff_paths(path, base)?;
        RelativePathBuf::from_path(path_diff).ok()
    }
}

struct ConfigError {
    source_file: RelativePathBuf,
    error: Value,
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let source_file = self.source_file.to_string();
        let content = BTreeMap::from([(source_file, &self.error)]);
        let output =
            serde_yaml::to_string(&content).unwrap_or(String::from("Failed to convert to YAML\n"));
        write!(f, "{}", output)
    }
}

fn report_error(source_file: RelativePathBuf, error: Value) -> ConfigError {
    ConfigError { source_file, error }
}

fn report_error_message(source_file: RelativePathBuf, msg: &str) -> ConfigError {
    ConfigError {
        source_file,
        error: Value::String(String::from(msg)),
    }
}

fn test_cases_errors(test_cases: &TestCases) -> Option<Value> {
    let mut contents = BTreeMap::new();

    let requirements = requirements_map(&test_cases.requirements);
    if requirements.len() > 0 {
        contents.insert("requirements", serde_yaml::to_value(requirements).ok()?);
    }

    if let Some(validation_errors) = validation_errors_map(&test_cases.validation_errors) {
        contents.insert("validation-errors", validation_errors);
    }

    if contents.len() > 0 {
        serde_yaml::to_value(contents).ok()
    } else {
        None
    }
}

fn requirements_map(requirements: &TestConfigData) -> BTreeMap<&str, BTreeMap<String, String>> {
    let mut contents = BTreeMap::new();

    let any_files_missing = requirements.any_missing_file_requirements();
    let files = requirements.file_requirements();
    if any_files_missing && files.len() > 0 {
        contents.insert(
            "files",
            files
                .into_iter()
                .map(|(x, y)| (x, show_presence(y)))
                .collect(),
        );
    }

    let any_env_missing = requirements.any_missing_env_requirements();
    let env = requirements.env_requirements();
    if any_env_missing && env.len() > 0 {
        contents.insert(
            "env",
            env.into_iter()
                .map(|(x, y)| (x, show_presence(y)))
                .collect(),
        );
    }

    contents
}

fn validation_errors_map(
    validation_errors: &Vec<(TestId, BTreeSet<TestCaseValidationError>)>,
) -> Option<Value> {
    if validation_errors.len() == 1 {
        let (maybe_root, errs) = &validation_errors[0];
        if maybe_root.is_root() {
            let contents = errs.iter().map(show_validation_error).collect::<Vec<_>>();
            return Some(serde_yaml::to_value(contents).unwrap_or(Value::Null));
        }
    }

    let mut contents = BTreeMap::new();

    for (test_id, errs) in validation_errors {
        contents.insert(
            test_id.to_string(),
            errs.iter().map(show_validation_error).collect::<Vec<_>>(),
        );
    }

    if contents.len() > 0 {
        Some(serde_yaml::to_value(contents).unwrap_or(Value::Null))
    } else {
        None
    }
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
        TestCaseValidationError::ExpectationRequired => {
            String::from("At least one expectation is required")
        }
    }
}

fn show_presence(value: bool) -> String {
    String::from(if value { "✔️" } else { "❌" })
}
