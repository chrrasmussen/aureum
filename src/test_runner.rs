use crate::tap_format;
use crate::test_case::{self, RunError, TestCase, TestResult};
use rayon::prelude::*;

pub struct ReportConfig {
    pub number_of_tests: usize,
}

pub fn report_start(report_config: &ReportConfig) {
    tap_format::print_version();
    tap_format::print_plan(1, report_config.number_of_tests);
}

pub fn run_test_cases(
    report_config: &ReportConfig,
    test_cases: &[TestCase],
    run_in_parallel: bool,
) -> bool {
    let run = |(i, test_case)| -> bool {
        let result = test_case::run(test_case);
        report_test_result(report_config, i, test_case, result)
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
