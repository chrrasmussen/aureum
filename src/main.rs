mod tap_format;
mod test_case;
mod test_config;

use clap::Parser;
use glob::glob;
use rayon::prelude::*;
use std::collections::BTreeSet;
use std::process::exit;
use std::{fs, io, path::PathBuf};
use test_case::{RunError, TestCase, TestResult};

const EXIT_CODE_ON_FAILURE: i32 = 1;

/// Golden test runner
#[derive(Parser)]
struct Args {
    /// Paths to test configs
    #[structopt(required = true)]
    paths: Vec<String>,
}

struct ReportConfig {
    number_of_tests: usize,
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

    let report_config = ReportConfig {
        number_of_tests: test_cases.len(),
    };

    report_start(&report_config);
    let all_successful = run_test_cases(&report_config, &test_cases, false);

    // TODO: Print failing configs

    if all_successful == false {
        exit(EXIT_CODE_ON_FAILURE)
    }
}

fn run_test_cases(
    report_config: &ReportConfig,
    test_cases: &[TestCase],
    run_in_parallel: bool,
) -> bool {
    let run = |(i, test_case)| -> bool {
        report_test_result(
            report_config,
            i,
            test_case,
            test_case::run(test_case.clone()),
        )
    };

    if run_in_parallel {
        test_cases
            .par_iter()
            .enumerate()
            .map(run)
            .reduce(|| true, |x, y| x && y)
    } else {
        test_cases
            .iter()
            .enumerate()
            .map(run)
            .fold(true, |x, y| x && y)
    }
}

fn report_start(report_config: &ReportConfig) {
    tap_format::print_version();
    tap_format::print_plan(1, report_config.number_of_tests);
}

fn report_test_result(
    report_config: &ReportConfig,
    index: usize,
    test_case: &TestCase,
    result: Result<TestResult, RunError>,
) -> bool {
    let test_number_indent_level = report_config.number_of_tests.to_string().len();
    print_test_case_result(index + 1, &test_case, &result, test_number_indent_level);

    if let Ok(result) = &result {
        if test_case::expectations_fulfilled(result) {
            return true;
        }
    }

    false
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
    result: &Result<TestResult, RunError>,
    indent_level: usize,
) {
    let is_success: bool;
    match result {
        Ok(test_result) => is_success = test_case::expectations_fulfilled(test_result),
        Err(_) => is_success = false,
    }

    let message: String;
    if let Some(description) = &case.description {
        message = format!(
            "{} # {}",
            case.source_file.display().to_string(),
            description,
        );
    } else {
        message = format!("{}", case.source_file.display().to_string(),);
    }

    if is_success {
        tap_format::print_ok(test_number, &message, indent_level)
    } else {
        tap_format::print_not_ok(test_number, &message, "", indent_level)
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
