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
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::exit;
use test_id_container::TestIdContainer;
use test_runner::{ReportConfig, ReportFormat, TestStatus};

const EXIT_CODE_ON_FAILURE: i32 = 1;

fn main() {
    let args = cli::parse();

    let test_files = expand_test_paths(&args.paths)
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    if test_files.is_empty() {
        eprintln!("No test configs found for the given paths");
        exit(EXIT_CODE_ON_FAILURE);
    }

    let mut all_test_cases = vec![];
    let mut failing_configs = vec![];

    for test_file in test_files {
        match test_config::test_cases_from_file(&test_file) {
            test_config::TestConfigResult::FailedToReadFile(_err) => {
                failing_configs.push(test_file.clone());
            }
            test_config::TestConfigResult::FailedToParseTestConfig(_err) => {
                failing_configs.push(test_file.clone());
            }
            test_config::TestConfigResult::PartialSuccess {
                requirements: _,
                validation_errors: _,
            } => {
                // TODO: Handle requirements
                failing_configs.push(test_file.clone());
            }
            test_config::TestConfigResult::Success {
                requirements: _,
                test_cases,
            } => {
                // TODO: Handle requirements
                all_test_cases.extend(test_cases);
            }
        }
    }

    for test_config_path in &failing_configs {
        eprintln!(
            "{}: Unable to generate test cases for test config",
            test_config_path.display()
        );
    }

    let report_config = ReportConfig {
        number_of_tests: all_test_cases.len(),
        format: get_report_format(&args),
    };

    test_runner::report_start(&report_config);
    let run_results =
        test_runner::run_test_cases(&report_config, &all_test_cases, args.run_tests_in_parallel);
    test_runner::report_summary(&report_config, &run_results);

    let all_tests_passed = run_results
        .iter()
        .fold(true, |acc, t| acc && t.test_status == TestStatus::Passed);
    if failing_configs.is_empty() == false || all_tests_passed == false {
        exit(EXIT_CODE_ON_FAILURE)
    }
}

pub fn expand_test_paths(test_paths: &[TestPath]) -> BTreeMap<PathBuf, TestIdContainer> {
    let mut test_files = BTreeMap::new();

    for test_path in test_paths {
        match test_path {
            TestPath::Pipe => {} // Skip
            TestPath::Glob(path) => {
                // TODO: Handle error case
                if let Ok(found_test_files) = locate_test_files(path.as_str()) {
                    for found_test_file in found_test_files {
                        test_files.insert(found_test_file, TestIdContainer::full());
                    }
                }
            }
            TestPath::SpecificFile { file_path, test_id } => {
                test_files
                    .entry(file_path.clone())
                    .and_modify(|test_ids: &mut TestIdContainer| {
                        test_ids.add(test_id.clone());
                    })
                    .or_insert(TestIdContainer::empty());
            }
        }
    }

    test_files
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
