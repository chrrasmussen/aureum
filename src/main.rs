use std::fs;
use std::io::{Write, Read};
use std::process::{Command, Stdio};
use std::str;
use serde::Deserialize;
use clap::Parser;

/// Golden test runner
#[derive(Parser, Debug)]
struct Args {
   /// Path to test config
   path: String,
}


#[derive(Debug, Deserialize)]
struct TestConfig {
    test_program: String,
    test_arguments: Vec<String>,
    test_stdin: String,
    expected_stdout: Option<String>,
    expected_stderr: Option<String>,
    expected_exit_code: Option<i32>,
}

fn main() {
    let args = Args::parse();

    let toml_str = fs::read_to_string(args.path)
        .expect("Should have been able to read the file");
    let test_config: TestConfig = toml::from_str(&toml_str).unwrap();

    let mut cmd = Command::new(test_config.test_program);
    cmd.args(test_config.test_arguments);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(test_config.test_stdin.as_bytes()).unwrap();
    drop(stdin);

    let mut stdout = child.stdout.take().unwrap();
    let mut buf: Vec<u8> = vec![];
    stdout.read_to_end(&mut buf).unwrap();

    let stdout_string = String::from_utf8(buf).unwrap();

    let mut stderr = child.stderr.take().unwrap();
    let mut buf: Vec<u8> = vec![];
    stderr.read_to_end(&mut buf).unwrap();

    let stderr_string = String::from_utf8(buf).unwrap();

    let exit_status = child.wait().unwrap();
    let exit_code = exit_status.code().unwrap();

    let is_success =
        test_config.expected_stdout.map(|x| { x == stdout_string }).unwrap_or(true) &&
        test_config.expected_stderr.map(|x| { x == stderr_string }).unwrap_or(true) &&
        test_config.expected_exit_code.map(|x| { x == exit_code }).unwrap_or(true);

    println!("{} - {} '{}' '{}'", if is_success { "ok" } else { "not ok" }, exit_code, stdout_string, stderr_string);
}
