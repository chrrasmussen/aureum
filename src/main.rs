mod tap_format;
mod test_case;
mod test_config;

use clap::Parser;
use glob::glob;
use std::collections::BTreeSet;
use std::process::exit;
use std::{fs, io, path::PathBuf};
use test_case::{TestCase, TestError, TestResult};

const EXIT_CODE_ON_FAILURE: i32 = 1;

/// Golden test runner
#[derive(Parser)]
struct Args {
    /// Paths to test configs
    #[structopt(required = true)]
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let mut test_files = BTreeSet::new();
    for path in &args.paths {
        locate_test_files(path, &mut test_files);
    }

    let mut test_cases = vec![];
    let mut failing_configs = vec![];

    for test_file in test_files {
        match test_cases_from_file(&test_file) {
            Ok(sub_tests) => test_cases.extend(sub_tests),
            Err(err) => failing_configs.push((test_file.clone(), err)),
        }
    }

    tap_format::print_version();
    tap_format::print_plan(1, test_cases.len());

    let mut any_failing_tests = false;

    for (i, test_case) in test_cases.iter().enumerate() {
        let test_result = test_case::run(&test_case);

        // Check if any tests have failed
        if let Ok(result) = &test_result {
            if result.is_success == false {
                any_failing_tests = true
            }
        }

        print_test_case_result(i + 1, &test_case, &test_result);
    }

    // TODO: Print failing configs

    if any_failing_tests {
        exit(EXIT_CODE_ON_FAILURE)
    }
}

enum TestFileError {
    FailedToParseTestConfig(test_config::TestConfigError),
    FailedToReadTestCases(test_config::TestConfigError),
    IOError(io::Error),
}

fn test_cases_from_file(test_file: &PathBuf) -> Result<Vec<TestCase>, TestFileError> {
    let toml_content = fs::read_to_string(test_file).map_err(TestFileError::IOError)?;
    let test_config =
        test_config::from_str(&toml_content).map_err(TestFileError::FailedToParseTestConfig)?;
    test_config
        .to_test_cases(test_file)
        .map_err(TestFileError::FailedToReadTestCases)
}

fn print_test_case_result(
    test_number: usize,
    case: &TestCase,
    result: &Result<TestResult, TestError>,
) {
    let is_success: bool;
    match result {
        Ok(test_result) => is_success = test_result.is_success,
        Err(_) => is_success = false,
    }

    let message = format!("{} [{}]", case.name, case.source_file.display().to_string());
    if is_success {
        tap_format::print_ok(test_number, &message)
    } else {
        tap_format::print_not_ok(test_number, &message, "")
    }
}

fn locate_test_files(path: &str, test_files: &mut BTreeSet<PathBuf>) {
    // Skip invalid patterns (Should it warn the user?)
    if let Ok(entries) = glob(path) {
        for entry in entries {
            if let Ok(e) = entry {
                if e.is_file() {
                    let test_file: PathBuf;
                    if e.is_relative() {
                        test_file = PathBuf::from(".").join(e)
                    } else {
                        test_file = e
                    }

                    test_files.insert(test_file);
                } else if e.is_dir() {
                    // Look for `.au.toml` files in directory (recursively)
                    if let Some(search_path) = e.join("**/*.au.toml").to_str() {
                        locate_test_files(search_path, test_files)
                    }
                }
            }
        }
    }
}
