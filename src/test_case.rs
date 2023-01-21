use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct TestCase {
    pub source_file: PathBuf,
    pub description: Option<String>,
    pub program: String,
    pub arguments: Vec<String>,
    pub stdin: Option<String>,
    pub expected_stdout: Option<String>,
    pub expected_stderr: Option<String>,
    pub expected_exit_code: Option<i32>,
}

pub struct TestResult {
    pub stdout: ValueComparison<String>,
    pub stderr: ValueComparison<String>,
    pub exit_code: ValueComparison<i32>,
}

pub enum ValueComparison<T> {
    NotChecked,
    Matches(T),
    Diff { expected: T, got: T },
}

impl<T> ValueComparison<T> {
    pub fn is_success(&self) -> bool {
        match self {
            Self::NotChecked => true,
            Self::Matches(_) => true,
            Self::Diff {
                expected: _,
                got: _,
            } => false,
        }
    }
}

#[derive(Debug)]
pub enum RunError {
    FailedToDecodeUtf8,
    MissingExitCode,
    IOError(io::Error),
}

pub fn run(test_case: &TestCase) -> Result<TestResult, RunError> {
    let current_dir = test_case.source_file.parent().unwrap_or(Path::new("."));

    let mut cmd = Command::new(&test_case.program);
    cmd.current_dir(current_dir);
    cmd.args(&test_case.arguments);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(RunError::IOError)?;

    if let Some(stdin_string) = &test_case.stdin {
        let mut stdin = child
            .stdin
            .take()
            .expect("Stdin should be configured to pipe");
        stdin
            .write_all(stdin_string.as_bytes())
            .map_err(RunError::IOError)?;
    }

    let stdout = read_pipe_to_string(
        &mut child
            .stdout
            .take()
            .expect("Stdout should be configured to pipe"),
    )?;
    let stderr = read_pipe_to_string(
        &mut child
            .stderr
            .take()
            .expect("Stderr should be configured to pipe"),
    )?;

    let exit_status = child.wait().map_err(RunError::IOError)?;
    let exit_code = exit_status
        .code()
        .map_or(Err(RunError::MissingExitCode), Ok)?;

    Ok(TestResult {
        stdout: compare_result(&test_case.expected_stdout, stdout),
        stderr: compare_result(&test_case.expected_stderr, stderr),
        exit_code: compare_result(&test_case.expected_exit_code, exit_code),
    })
}

pub fn expectations_fulfilled(result: &TestResult) -> bool {
    result.stdout.is_success() && result.stderr.is_success() && result.exit_code.is_success()
}

fn compare_result<T: PartialEq + Clone>(expected: &Option<T>, got: T) -> ValueComparison<T> {
    if let Some(expected) = expected {
        if expected == &got {
            ValueComparison::Matches(got)
        } else {
            ValueComparison::Diff { expected: expected.clone(), got }
        }
    } else {
        ValueComparison::NotChecked
    }
}

fn read_pipe_to_string<T>(pipe: &mut T) -> Result<String, RunError>
where
    T: Read,
{
    let mut buf: Vec<u8> = vec![];
    pipe.read_to_end(&mut buf).map_err(RunError::IOError)?;
    String::from_utf8(buf).map_or(Err(RunError::FailedToDecodeUtf8), Ok)
}
