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
use std::collections::{BTreeMap, BTreeSet};
use std::process::exit;
use std::{fs, io, path::PathBuf};
use test_case::TestCase;
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

    let mut test_cases = vec![];
    let mut failing_configs = vec![];

    for test_file in test_files {
        match test_cases_from_file(&test_file) {
            Ok(sub_tests) => test_cases.extend(sub_tests),
            Err(err) => failing_configs.push((test_file.clone(), err)),
        }
    }

    for (test_config_path, _err) in &failing_configs {
        eprintln!(
            "{}: Unable to parse test config",
            test_config_path.display()
        );
    }

    let report_config = ReportConfig {
        number_of_tests: test_cases.len(),
        format: get_report_format(&args),
    };

    test_runner::report_start(&report_config);
    let run_results =
        test_runner::run_test_cases(&report_config, &test_cases, args.run_tests_in_parallel);
    test_runner::report_summary(&report_config, &run_results);

    let all_tests_passed = run_results
        .iter()
        .fold(true, |acc, t| acc && t.test_status == TestStatus::Passed);
    if failing_configs.is_empty() == false || all_tests_passed == false {
        exit(EXIT_CODE_ON_FAILURE)
    }
}

enum TestFileError {
    FailedToParseTestConfig(test_config::ParseTestConfigError),
    FailedToReadTestCases(test_config::TestConfigError),
    IOError(io::Error),
}

fn get_report_format(args: &Args) -> ReportFormat {
    match args.output_format {
        OutputFormat::Summary => ReportFormat::Summary {
            show_all_tests: args.show_all_tests,
        },
        OutputFormat::Tap => ReportFormat::Tap,
    }
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
                    test_files.insert(e);
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

pub fn expand_test_paths(test_paths: &[TestPath]) -> BTreeMap<PathBuf, TestIdContainer> {
    let mut test_files = BTreeMap::new();

    for test_path in test_paths {
        match test_path {
            TestPath::Pipe => {} // Skip
            TestPath::Glob(path) => {
                let mut glob_test_files = BTreeSet::new();
                locate_test_files(path.as_str(), &mut glob_test_files);

                for glob_test_file in glob_test_files {
                    test_files.insert(glob_test_file, TestIdContainer::full());
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
