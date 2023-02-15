use crate::utils::string;

pub fn text_block(content: &str) -> String {
    let prefixed_content = string::indent_with("│ ", content);

    if content.ends_with("\n") {
        format!("╭\n{}╰", prefixed_content)
    } else {
        format!("╭\n{}\n╰ (No newline at end)", prefixed_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_text_block_empty() {
        let expected = indoc! {"
            ╭
            │ 
            ╰ (No newline at end)"};

        assert_eq!(text_block(""), expected);
    }

    #[test]
    fn test_text_block_only_newline() {
        let expected = indoc! {"
            ╭
            │ 
            ╰"};

        assert_eq!(text_block("\n"), expected);
    }

    #[test]
    fn test_text_block_single_line_no_newline() {
        let expected = indoc! {"
            ╭
            │ foo
            ╰ (No newline at end)"};

        assert_eq!(text_block("foo"), expected);
    }

    #[test]
    fn test_text_block_single_line_with_newline() {
        let expected = indoc! {"
            ╭
            │ foo
            ╰"};

        assert_eq!(text_block("foo\n"), expected);
    }

    #[test]
    fn test_text_block_multiple_lines_no_newline() {
        let expected = indoc! {"
            ╭
            │ line 1
            │ line 2
            ╰ (No newline at end)"};

        assert_eq!(text_block("line 1\nline 2"), expected);
    }

    #[test]
    fn test_text_block_multiple_lines_with_newline() {
        let expected = indoc! {"
            ╭
            │ line 1
            │ line 2
            ╰"};

        assert_eq!(text_block("line 1\nline 2\n"), expected);
    }

    #[test]
    fn test_text_block_multiple_lines_including_empty_lines() {
        let expected = indoc! {"
            ╭
            │ line 1
            │ 
            │ line 3
            ╰"};

        assert_eq!(text_block("line 1\n\nline 3\n"), expected);
    }
}
