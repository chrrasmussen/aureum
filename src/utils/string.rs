pub fn indent_lines(input: &str, indent_level: usize) -> String {
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
