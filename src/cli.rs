use crate::test_id::TestId;
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;

pub fn parse() -> Args {
    Args::parse()
}

/// Golden test runner for executables
#[derive(Parser)]
pub struct Args {
    /// Paths to test configs
    #[arg(required = true)]
    pub paths: Vec<TestPath>,

    /// Options: summary, tap
    #[arg(long, default_value = "summary")]
    pub output_format: OutputFormat,

    /// Show all tests in summary, regardless of test status
    #[arg(long)]
    pub show_all_tests: bool,

    /// Run tests in parallel
    #[arg(long)]
    pub run_tests_in_parallel: bool,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum TestPath {
    Pipe,
    Glob(String),
    SpecificFile { file_path: PathBuf, test_id: TestId },
}

impl FromStr for TestPath {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            Ok(Self::Pipe)
        } else if let Some((prefix, suffix)) = s.split_once(":") {
            let path = PathBuf::from(prefix);

            if path.is_file() {
                Ok(Self::SpecificFile {
                    file_path: path,
                    test_id: TestId::from(suffix),
                })
            } else {
                Err("Invalid path to test config")
            }
        } else {
            Ok(Self::Glob(s.to_owned()))
        }
    }
}

#[derive(Clone)]
pub enum OutputFormat {
    Summary,
    Tap,
}

impl FromStr for OutputFormat {
    type Err = &'static str;

    fn from_str(format: &str) -> Result<Self, Self::Err> {
        match format {
            "summary" => Ok(Self::Summary),
            "tap" => Ok(Self::Tap),
            _ => Err("Invalid output format"),
        }
    }
}
