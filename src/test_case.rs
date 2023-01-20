use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

pub struct TestOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct TestResult {
    pub is_success: bool,
    pub output: TestOutput,
}

#[derive(Debug)]
pub enum TestError {
    FailedToDecodeUtf8,
    MissingExitCode,
    IOError(io::Error),
}

pub fn run(test_case: &TestCase) -> Result<TestResult, TestError> {
    let current_dir = test_case.source_file.parent().unwrap_or(Path::new("."));

    let mut cmd = Command::new(&test_case.program);
    cmd.current_dir(current_dir);
    cmd.args(&test_case.arguments);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(TestError::IOError)?;

    if let Some(stdin_string) = &test_case.stdin {
        let mut stdin = child
            .stdin
            .take()
            .expect("Stdin should be configured to pipe");
        stdin
            .write_all(stdin_string.as_bytes())
            .map_err(TestError::IOError)?;
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

    let exit_status = child.wait().map_err(TestError::IOError)?;
    let exit_code = exit_status
        .code()
        .map_or(Err(TestError::MissingExitCode), Ok)?;

    let output = TestOutput {
        stdout,
        stderr,
        exit_code,
    };

    let stdout_result = test_case
        .expected_stdout
        .as_ref()
        .map_or(true, |x| x == &output.stdout);
    let stderr_result = test_case
        .expected_stderr
        .as_ref()
        .map_or(true, |x| x == &output.stderr);
    let exit_code_result = test_case
        .expected_exit_code
        .map_or(true, |x| x == output.exit_code);

    let is_success = stdout_result && stderr_result && exit_code_result;

    Ok(TestResult { is_success, output })
}

fn read_pipe_to_string<T>(pipe: &mut T) -> Result<String, TestError>
where
    T: Read,
{
    let mut buf: Vec<u8> = vec![];
    pipe.read_to_end(&mut buf).map_err(TestError::IOError)?;
    String::from_utf8(buf).map_or(Err(TestError::FailedToDecodeUtf8), Ok)
}
