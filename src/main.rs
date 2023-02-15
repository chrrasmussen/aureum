mod cli;

use aureum::test_id_container::TestIdContainer;
use aureum::test_runner::{ReportConfig, ReportFormat};
use aureum::toml_config::TomlConfigError;
use cli::error;
use cli::{Args, OutputFormat, TestPath};
use glob::glob;
use pathdiff;
use relative_path::RelativePathBuf;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::exit;

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
        match aureum::toml_config::test_cases_from_file(&source_file) {
            Ok(result) => {
                let errors = error::test_cases_errors(&result);
                if errors.len() > 0 {
                    failed_configs.push(error::report_error(source_file, errors));
                }

                all_test_cases.extend(result.test_cases);
            }
            Err(err) => {
                let msg = match err {
                    TomlConfigError::FailedToReadFile(_) => "Failed to read file",
                    TomlConfigError::FailedToParseTomlConfig(_) => "Failed to parse config file",
                };
                failed_configs.push(error::report_error_message(source_file, msg));
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

    let run_results = aureum::test_runner::run_test_cases(
        &report_config,
        &all_test_cases,
        args.run_tests_in_parallel,
    );

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
