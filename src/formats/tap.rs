use crate::utils::string;

pub fn print_version() {
    println!("TAP version 14")
}

pub fn print_plan(start: usize, end: usize) {
    println!("{}..{}", start, end)
}

pub fn print_ok(test_number: usize, message: &str, indent_level: usize) {
    println!(
        "ok     {:>indent$} - {}",
        test_number,
        message,
        indent = indent_level
    )
}

pub fn print_not_ok(test_number: usize, message: &str, diagnostics: &str, indent_level: usize) {
    println!(
        "not ok {:>indent$} - {}",
        test_number,
        message,
        indent = indent_level
    );
    if diagnostics.is_empty() == false {
        print_diagnostics(diagnostics)
    }
}

pub fn print_diagnostics(diagnostics: &str) {
    let code_block = format!("---\n{}...", diagnostics);
    println!("{}", string::indent_lines(&code_block, 2));
}

#[allow(dead_code)]
pub fn print_bail_out(message: &str) {
    println!("Bail out! {}", message)
}
