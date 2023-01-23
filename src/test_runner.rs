use crate::tap_format;
use crate::test_case::{self, RunError, TestCase, TestResult};
use rayon::prelude::*;

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
        let result = test_case::run(test_case);
        report_test_case(report_config, i, test_case, &result);

        if let Ok(result) = &result {
            if test_case::expectations_fulfilled(result) {
                return vec![TestSummary {
                    test_case: test_case.clone(),
                    test_status: TestStatus::Passed,
                }];
            }
        }

        vec![TestSummary {
            test_case: test_case.clone(),
            test_status: TestStatus::Failed,
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
    result: &Result<TestResult, RunError>,
) {
    match report_config.format {
        ReportFormat::Summary => {}
        ReportFormat::Tap => {
            let test_number_indent_level = report_config.number_of_tests.to_string().len();
            print_test_case_result(index + 1, &test_case, &result, test_number_indent_level);
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
    result: &Result<TestResult, RunError>,
    indent_level: usize,
) {
    let is_success: bool;
    match result {
        Ok(test_result) => is_success = test_case::expectations_fulfilled(test_result),
        Err(_) => is_success = false,
    }

    let message: String;
    if let Some(description) = &test_case.description {
        message = format!("{} # {}", test_case.id(), description);
    } else {
        message = format!("{}", test_case.id());
    }

    if is_success {
        tap_format::print_ok(test_number, &message, indent_level)
    } else {
        tap_format::print_not_ok(test_number, &message, "", indent_level)
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
