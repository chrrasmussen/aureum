mod test_case;
mod test_config;

use clap::Parser;
use glob::glob;
use std::{fs, path::PathBuf};

/// Golden test runner
#[derive(Parser)]
struct Args {
    /// Path to test config
    path: String,
}

fn main() {
    let args = Args::parse();

    let mut test_files = vec![];
    locate_test_files(&args.path, &mut test_files);

    for test_file in test_files {
        let toml_str =
            fs::read_to_string(&test_file).expect("Should have been able to read the file");
        let test_config = test_config::from_str(&toml_str).unwrap();

        let test_case = test_config.to_test_case(&test_file).unwrap();
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
        )
    }
}

fn locate_test_files(path: &str, test_files: &mut Vec<PathBuf>) {
    // Skip invalid patterns (Should it warn the user?)
    if let Ok(entries) = glob(path) {
        for entry in entries {
            if let Ok(e) = entry {
                if e.is_file() {
                    test_files.push(e)
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
