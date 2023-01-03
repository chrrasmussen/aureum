use std::fs;
use std::io::{Write, Read};
use std::process::{Command, Stdio};
use std::str;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TestConfig {
    test_program: String,
    test_arguments: Vec<String>,
    test_stdin: String,
    expected_stdout: String,
}

fn main() {
    let toml_str = fs::read_to_string("examples/bash.toml")
        .expect("Should have been able to read the file");
    let test_config: TestConfig = toml::from_str(&toml_str).unwrap();

    println!("{:#?}", test_config);

    let mut cmd = Command::new(test_config.test_program);
    cmd.args(test_config.test_arguments);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();

    println!("Process started");

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(test_config.test_stdin.as_bytes()).unwrap();
    drop(stdin);

    println!("Process got stdin");

    let mut stdout = child.stdout.take().unwrap();
    let mut buf: Vec<u8> = vec![];
    stdout.read_to_end(&mut buf).unwrap();

    let stdout_string = String::from_utf8(buf).unwrap();

    println!("Process got stdout");

    let mut stderr = child.stderr.take().unwrap();
    let mut buf: Vec<u8> = vec![];
    stderr.read_to_end(&mut buf).unwrap();

    let stderr_string = String::from_utf8(buf).unwrap();

    println!("Process got stderr");

    let exit_status = child.wait().unwrap();

    println!("{} - {} '{}' '{}'", if stdout_string == test_config.expected_stdout { "ok" } else { "not ok" }, exit_status.code().unwrap(), stdout_string, stderr_string);
}
