mod cli;

use aureum::test_runner::{ReportConfig, ReportFormat};
use cli::file;
use cli::report;
use cli::{Args, OutputFormat};
use std::env;
use std::process::exit;

const TEST_FAILURE_EXIT_CODE: i32 = 1;
const INVALID_USER_INPUT_EXIT_CODE: i32 = 2;

fn main() {
    let args = cli::parse();

    let current_dir = env::current_dir().expect("Current directory must be available");

    let source_files = file::expand_test_paths(&args.paths, &current_dir)
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    if source_files.is_empty() {
        eprintln!("error: No config files found for the given paths");
        exit(INVALID_USER_INPUT_EXIT_CODE);
    }

    if args.verbose {
        report::print_files_found(&source_files);
    }

    let mut all_test_cases = vec![];
    let mut any_failed_configs = false;

    for source_file in source_files {
        match aureum::toml_config::parse_toml_config(&source_file) {
            Ok(config) => {
                let any_issues = report::any_issues_in_toml_config(&config);
                if any_issues || args.verbose {
                    report::print_config_details(source_file, &config, args.verbose);

                    if any_issues {
                        any_failed_configs = true;
                    }
                }

                all_test_cases.extend(config.test_cases);
            }
            Err(error) => {
                report::print_toml_config_error(source_file, error);
                any_failed_configs = true;
            }
        }
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

    if any_failed_configs {
        eprintln!("Some config files contain errors (See above)");
    }

    let all_tests_passed = run_results.iter().all(|t| t.is_success());

    if any_failed_configs || !all_tests_passed {
        exit(TEST_FAILURE_EXIT_CODE)
    }
}

fn get_report_format(args: &Args) -> ReportFormat {
    match args.output_format {
        OutputFormat::Summary => ReportFormat::Summary {
            show_all_tests: args.show_all_tests,
        },
        OutputFormat::Tap => ReportFormat::Tap,
    }
}
