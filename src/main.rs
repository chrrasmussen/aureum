mod tap_format;
mod test_case;
mod test_config;
mod test_runner;

use clap::Parser;
use glob::glob;
use std::collections::BTreeSet;
use std::process::exit;
use std::{fs, io, path::PathBuf};
use test_case::TestCase;

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

    if test_files.is_empty() {
        eprintln!("No test configs found for the given path(s)");
        exit(EXIT_CODE_ON_FAILURE);
    }

    let mut test_cases = vec![];
    let mut failing_configs = vec![];

    for test_file in test_files {
        match test_cases_from_file(&test_file) {
            Ok(sub_tests) => test_cases.extend(sub_tests),
            Err(err) => failing_configs.push((test_file.clone(), err)),
        }
    }

    for (test_config_path, _err) in &failing_configs {
        eprintln!("{}: Unable to parse test config", test_config_path.display());
    }

    let report_config = test_runner::ReportConfig {
        number_of_tests: test_cases.len(),
    };

    test_runner::report_start(&report_config);
    let all_tests_passed = test_runner::run_test_cases(&report_config, &test_cases, false);

    if failing_configs.is_empty() == false || all_tests_passed == false {
        exit(EXIT_CODE_ON_FAILURE)
    }
}

enum TestFileError {
    FailedToParseTestConfig(test_config::ParseTestConfigError),
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
