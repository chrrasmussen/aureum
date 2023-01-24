use clap::Parser;
use std::str::FromStr;

pub fn parse() -> Args {
    Args::parse()
}

/// Golden test runner
#[derive(Parser)]
pub struct Args {
    /// Paths to test configs
    #[arg(required = true)]
    pub paths: Vec<String>,

    /// Options: summary, tap
    #[arg(long, default_value = "summary")]
    pub output_format: OutputFormat,

    /// Show all tests in summary, regardless of test status
    #[arg(long)]
    pub show_all_tests: bool,

    /// Run tests in parallell
    #[arg(long)]
    pub run_tests_in_parallell: bool,
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
