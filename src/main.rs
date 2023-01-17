mod test_case;
mod test_config;

use clap::Parser;
use glob::glob;
use std::{fs, io, path::PathBuf};
use test_case::{TestCase, TestError, TestResult};

/// Golden test runner
#[derive(Parser)]
struct Args {
    /// Path to test config
    path: String,
}

fn main() {
    let args = Args::parse();

    let mut test_files = vec![];
    locate_test_files(&args.path, &mut test_files);

    let mut test_cases = vec![];
    let mut failing_configs = vec![];

    for test_file in test_files {
        match test_case_from_file(&test_file) {
            Ok(test_case) => test_cases.push(test_case),
            Err(err) => failing_configs.push((test_file.clone(), err)),
        }
    }

    for test_case in test_cases {
        let test_result = test_case::run(&test_case);

        print_test_result(&test_case, &test_result);
    }

    // TODO: Print failing configs
}

enum TestFileError {
    FailedToParseTestConfig(test_config::TestConfigError),
    FailedToReadTestCases(test_config::TestConfigError),
    IOError(io::Error),
}

fn test_case_from_file(test_file: &PathBuf) -> Result<TestCase, TestFileError> {
    let toml_content = fs::read_to_string(test_file).map_err(TestFileError::IOError)?;
    let test_config =
        test_config::from_str(&toml_content).map_err(TestFileError::FailedToParseTestConfig)?;
    test_config
        .to_test_case(test_file)
        .map_err(TestFileError::FailedToReadTestCases)
}

fn print_test_result(_case: &TestCase, result: &Result<TestResult, TestError>) {
    if let Ok(result) = result {
        println!(
            "{} - {} '{}' '{}'",
            if result.is_success { "ok" } else { "not ok" },
            result.output.exit_code,
            result.output.stdout,
            result.output.stderr
        )
    }
}

fn locate_test_files(path: &str, test_files: &mut Vec<PathBuf>) {
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

                    test_files.push(test_file)
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
