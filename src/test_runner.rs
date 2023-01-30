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

#[derive(PartialEq, Clone)]
pub enum TestStatus {
    Passed,
    Failed,
}

#[derive(Clone)]
pub struct RunResult {
    pub test_case: TestCase,
    pub test_status: TestStatus,
}

pub fn report_start(report_config: &ReportConfig) {
    match report_config.format {
        ReportFormat::Summary { show_all_tests: _ } => {}
        ReportFormat::Tap => {
            tap_format::print_version();
            tap_format::print_plan(1, report_config.number_of_tests);
        }
    }
}

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

    if run_in_parallel {
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
    }
}

fn report_test_case(
    report_config: &ReportConfig,
    index: usize,
    test_case: &TestCase,
    result: Result<TestResult, RunError>,
) {
    match report_config.format {
        ReportFormat::Summary { show_all_tests: _ } => {}
        ReportFormat::Tap => {
            let test_number_indent_level = report_config.number_of_tests.to_string().len();
            print_tap_result(index + 1, &test_case, result, test_number_indent_level);
        }
    }
}

pub fn report_summary(report_config: &ReportConfig, run_results: &[RunResult]) {
    match report_config.format {
        ReportFormat::Summary { show_all_tests } => {
            for run_result in run_results {
                let test_failed = run_result.test_status == TestStatus::Failed;
                if show_all_tests || test_failed {
                    print_summary_result(run_result);
                }
            }

            let number_of_failed_tests = run_results
                .iter()
                .filter(|t| t.test_status == TestStatus::Failed)
                .count();

            println!();
            println!("Completed running {} tests.", report_config.number_of_tests);

            if number_of_failed_tests == 0 {
                println!("All tests passed.")
            } else {
                println!("{} failures.", number_of_failed_tests)
            }
        }
        ReportFormat::Tap => {}
    }
}

// TAP HELPERS

fn print_tap_result(
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

// SUMMARY HELPERS

fn print_summary_result(run_result: &RunResult) {
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
