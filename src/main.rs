mod test_case;
mod test_config;

use clap::Parser;
use std::fs;

/// Golden test runner
#[derive(Parser)]
struct Args {
    /// Path to test config
    path: String,
}

fn main() {
    let args = Args::parse();

    let toml_str = fs::read_to_string(&args.path).expect("Should have been able to read the file");
    let test_config = test_config::from_str(&toml_str).unwrap();

    let test_case = test_config.to_test_case(&args.path).unwrap();
    let test_result = test_case::run(&test_case).unwrap();

    println!(
        "{} - {} '{}' '{}'",
        if test_result.is_success {
            "ok"
        } else {
            "not ok"
        },
        test_result.output.exit_code,
        test_result.output.stdout,
        test_result.output.stderr
    );
}
