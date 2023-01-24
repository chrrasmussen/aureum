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
    Summary,
    Tap,
}

#[derive(PartialEq, Clone)]
pub enum TestStatus {
    Passed,
    Failed,
}

#[derive(Clone)]
pub struct TestSummary {
    pub test_case: TestCase,
    pub test_status: TestStatus,
}

pub fn report_start(report_config: &ReportConfig) {
    match report_config.format {
        ReportFormat::Summary => {}
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
) -> Vec<TestSummary> {
    let run = |(i, test_case)| -> Vec<TestSummary> {
        let test_result = test_case::run(test_case);

        let is_success = test_result
            .as_ref()
            .map_or(false, |t| t.is_success())
            .clone();
        report_test_case(report_config, i, test_case, test_result);

        if is_success {
            return vec![TestSummary {
                test_case: test_case.clone(),
                test_status: TestStatus::Passed,
            }];
        } else {
            vec![TestSummary {
                test_case: test_case.clone(),
                test_status: TestStatus::Failed,
            }]
        }
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
        ReportFormat::Summary => {}
        ReportFormat::Tap => {
            let test_number_indent_level = report_config.number_of_tests.to_string().len();
            print_test_case_result(index + 1, &test_case, result, test_number_indent_level);
        }
    }
}

pub fn report_summary(report_config: &ReportConfig, test_summaries: &[TestSummary]) {
    match report_config.format {
        ReportFormat::Summary => {
            for test_summary in test_summaries {
                print_test_summary(test_summary);
            }

            let number_of_failed_tests = test_summaries
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

fn print_test_case_result(
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

fn print_test_summary(test_summary: &TestSummary) {
    let test_id = test_summary.test_case.id();

    let message: String;
    if let Some(description) = &test_summary.test_case.description {
        message = format!("{} - {}", test_id, description);
    } else {
        message = format!("{}", test_id);
    }

    let is_success = test_summary.test_status == TestStatus::Passed;
    if is_success {
        println!("✅ {}", message)
    } else {
        println!("❌ {}", message)
    }
}

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

    serde_yaml::to_string(&diagnostics).unwrap_or("Failed to convert to YAML".to_owned())
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
