use crate::tap_format;
use crate::test_case::{self, RunError, TestCase, TestResult, ValueComparison};
use rayon::prelude::*;
use serde_yaml::{self, Number, Value};
use std::collections::BTreeMap;

pub struct ReportConfig {
    pub number_of_tests: usize,
    pub format: ReportFormat,
}

pub enum ReportFormat {
    Summary { show_all_tests: bool },
    Tap,
}

#[derive(Clone)]
pub struct RunResult {
    pub test_case: TestCase,
    pub test_status: TestStatus,
}

#[derive(PartialEq, Clone)]
pub enum TestStatus {
    Passed,
    Failed,
}

// RUN TEST CASES

pub fn run_test_cases(
    report_config: &ReportConfig,
    test_cases: &[TestCase],
    run_in_parallel: bool,
) -> Vec<RunResult> {
    let run = |(i, test_case)| -> Vec<RunResult> {
        let result = test_case::run(test_case);

        let is_success = result.as_ref().map_or(false, |t| t.is_success());
        report_test_case(report_config, i, test_case, result);

        let test_status = if is_success {
            TestStatus::Passed
        } else {
            TestStatus::Failed
        };

        vec![RunResult {
            test_case: test_case.clone(),
            test_status,
        }]
    };

    report_start(report_config);

    let run_results = if run_in_parallel {
        test_cases
            .par_iter()
            .enumerate()
            .map(run)
            .reduce(|| vec![], |x, y| [x, y].concat())
    } else {
        test_cases
            .iter()
            .enumerate()
            .map(run)
            .fold(vec![], |x, y| [x, y].concat())
    };

    report_summary(&report_config, &run_results);

    run_results
}

// REPORTING

fn report_start(report_config: &ReportConfig) {
    match report_config.format {
        ReportFormat::Summary { show_all_tests: _ } => {
            summary_print_start(report_config.number_of_tests);
        }
        ReportFormat::Tap => {
            tap_print_start(report_config.number_of_tests);
        }
    }
}

fn report_test_case(
    report_config: &ReportConfig,
    index: usize,
    test_case: &TestCase,
    result: Result<TestResult, RunError>,
) {
    match report_config.format {
        ReportFormat::Summary { show_all_tests: _ } => {
            summary_print_test_case(result);
        }
        ReportFormat::Tap => {
            let test_number_indent_level = report_config.number_of_tests.to_string().len();
            tap_print_test_case(index + 1, &test_case, result, test_number_indent_level);
        }
    }
}

fn report_summary(report_config: &ReportConfig, run_results: &[RunResult]) {
    match report_config.format {
        ReportFormat::Summary { show_all_tests } => {
            summary_print_summary(report_config.number_of_tests, show_all_tests, run_results);
        }
        ReportFormat::Tap => {
            tap_print_summary();
        }
    }
}

// SUMMARY HELPERS

fn summary_print_start(number_of_tests: usize) {
    println!("Started running {} tests:", number_of_tests)
}

fn summary_print_test_case(result: Result<TestResult, RunError>) {
    match result {
        Ok(test_result) => {
            if test_result.is_success() {
                print!(".")
            } else {
                print!("F")
            }
        }
        Err(_) => {
            print!("F")
        }
    }
}

fn summary_print_summary(number_of_tests: usize, show_all_tests: bool, run_results: &[RunResult]) {
    println!(); // Add newline to dots

    let mut is_any_test_cases_printed = false;

    for run_result in run_results {
        let test_failed = run_result.test_status == TestStatus::Failed;
        if show_all_tests || test_failed {
            if is_any_test_cases_printed == false {
                println!();
                is_any_test_cases_printed = true;
            }

            summary_print_result(run_result);
        }
    }

    let number_of_failed_tests = run_results
        .iter()
        .filter(|t| t.test_status == TestStatus::Failed)
        .count();
    let number_of_passed_tests = number_of_tests - number_of_failed_tests;

    println!();
    println!(
        "Finished running tests: {} passed, {} failures",
        number_of_passed_tests, number_of_failed_tests,
    );
}

fn summary_print_result(run_result: &RunResult) {
    let test_id = run_result.test_case.id();

    let message: String;
    if let Some(description) = &run_result.test_case.description {
        message = format!("{} - {}", test_id, description);
    } else {
        message = format!("{}", test_id);
    }

    let is_success = run_result.test_status == TestStatus::Passed;
    if is_success {
        println!("✅ {}", message)
    } else {
        println!("❌ {}", message)
    }
}

// TAP HELPERS

fn tap_print_start(number_of_tests: usize) {
    tap_format::print_version();
    tap_format::print_plan(1, number_of_tests);
}

fn tap_print_test_case(
    test_number: usize,
    test_case: &TestCase,
    result: Result<TestResult, RunError>,
    indent_level: usize,
) {
    let message: String;
    if let Some(description) = &test_case.description {
        message = format!("{} # {}", test_case.id(), description);
    } else {
        message = format!("{}", test_case.id());
    }

    match result {
        Ok(test_result) => {
            if test_result.is_success() {
                tap_format::print_ok(test_number, &message, indent_level)
            } else {
                tap_format::print_not_ok(
                    test_number,
                    &message,
                    &format_test_result(test_result),
                    indent_level,
                )
            }
        }
        Err(_) => {
            tap_format::print_not_ok(test_number, &message, "Failed to run test", indent_level)
        }
    }
}

fn tap_print_summary() {}

// ERROR FORMATTING

fn format_test_result(test_result: TestResult) -> String {
    let mut diagnostics = BTreeMap::new();

    if let ValueComparison::Diff { expected, got } = test_result.stdout {
        diagnostics.insert("stdout", show_string_diff(expected, got));
    }

    if let ValueComparison::Diff { expected, got } = test_result.stderr {
        diagnostics.insert("stderr", show_string_diff(expected, got));
    }

    if let ValueComparison::Diff { expected, got } = test_result.exit_code {
        diagnostics.insert("exit-code", show_i32_diff(expected, got));
    }

    serde_yaml::to_string(&diagnostics).unwrap_or(String::from("Failed to convert to YAML"))
}

fn show_string_diff(expected: String, got: String) -> BTreeMap<&'static str, Value> {
    show_diff(Value::String(expected), Value::String(got))
}

fn show_i32_diff(expected: i32, got: i32) -> BTreeMap<&'static str, Value> {
    show_diff(
        Value::Number(Number::from(expected)),
        Value::Number(Number::from(got)),
    )
}

fn show_diff(expected: Value, got: Value) -> BTreeMap<&'static str, Value> {
    BTreeMap::from([("expected", expected), ("got", got)])
}
