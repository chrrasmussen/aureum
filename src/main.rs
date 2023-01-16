mod test_case;
mod test_config;

use clap::Parser;
use std::fs;
use test_case::{TestAssertion, TestOutput};
use test_config::TestConfig;

/// Golden test runner
#[derive(Parser)]
struct Args {
    /// Path to test config
    path: String,
}

fn main() {
    let args = Args::parse();

    let toml_str = fs::read_to_string(&args.path).expect("Should have been able to read the file");
    let test_config: TestConfig = toml::from_str(&toml_str).unwrap();

    let test_case = test_config.to_test_case(&args.path).unwrap();
    let test_output = test_case::run(&test_case).unwrap();

    let is_success = test_case
        .assertions
        .into_iter()
        .all(|assertion| check_assertion(&test_output, &assertion));

    println!(
        "{} - {} '{}' '{}'",
        if is_success { "ok" } else { "not ok" },
        test_output.exit_code,
        test_output.stdout,
        test_output.stderr
    );
}

fn check_assertion(output: &TestOutput, assertion: &TestAssertion) -> bool {
    match assertion {
        TestAssertion::AssertStdout(expected_stdout) => &output.stdout == expected_stdout,
        TestAssertion::AssertStderr(expected_stderr) => &output.stderr == expected_stderr,
        TestAssertion::AssertExitCode(expected_exit_code) => {
            &output.exit_code == expected_exit_code
        }
    }
}
