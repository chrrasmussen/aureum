pub fn print_plan(start: usize, end: usize) {
    println!("{}..{}", start, end)
}

pub fn print_ok(test_number: usize, message: &str) {
    println!("ok {} - {}", test_number, message)
}

pub fn print_not_ok(test_number: usize, message: &str, diagnostics: &str) {
    println!("not ok {} - {}", test_number, message);
    if diagnostics.is_empty() == false {
        print_diagnostics(diagnostics)
    }
}

pub fn print_diagnostics(diagnostics: &str) {
    let code_block = format!("---\n{}\n...", diagnostics);
    println!("{}", indent_lines(&code_block, 2));
}

pub fn print_bail_out(message: &str) {
    println!("Bail out! {}", message)
}

fn indent_lines(input: &str, indent_level: usize) -> String {
    let mut output = String::new();

    for (i, line) in input.lines().enumerate() {
        if i > 0 {
            output.push('\n')
        }

        let indented_line = format!("{:indent$}{}", "", line, indent = indent_level);
        output.push_str(&indented_line)
    }

    if input.ends_with('\n') {
        output.push('\n')
    }

    output
}
