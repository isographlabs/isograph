use std::collections::VecDeque;

use intern::string_key::Intern;
use prelude::Postfix;

pub(crate) fn intern_block_string_value<T: From<intern::string_key::StringKey>>(
    interior: &str,
) -> T {
    clean_block_string(interior).intern().to()
}

fn clean_block_string(source: &str) -> String {
    let common_indent = get_common_indent(source);

    let mut formatted_lines = source
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                line.to_string()
            } else {
                line.chars().skip(common_indent).collect::<String>()
            }
        })
        .collect::<VecDeque<String>>();

    while formatted_lines
        .front()
        .is_some_and(|line| line_is_whitespace(line))
    {
        formatted_lines.pop_front();
    }
    while formatted_lines
        .back()
        .is_some_and(|line| line_is_whitespace(line))
    {
        formatted_lines.pop_back();
    }

    let lines_vec: Vec<String> = formatted_lines.into_iter().collect();
    lines_vec.join("\n")
}

fn get_common_indent(source: &str) -> usize {
    let lines = source.lines().skip(1);
    let mut common_indent: Option<usize> = None;
    for line in lines {
        if let Some((first_index, _)) = line.match_indices(is_not_whitespace).next()
            && common_indent.is_none_or(|indent| first_index < indent)
        {
            common_indent = first_index.wrap_some()
        }
    }
    common_indent.unwrap_or(0)
}

fn line_is_whitespace(line: &str) -> bool {
    !line.contains(is_not_whitespace)
}

fn is_not_whitespace(c: char) -> bool {
    c != ' ' && c != '\t'
}

#[cfg(test)]
mod tests {
    use super::clean_block_string;

    #[test]
    fn a_block_string_dedents() {
        assert_eq!(clean_block_string("hi"), "hi");
        assert_eq!(clean_block_string("\n  hello\n  world\n"), "hello\nworld");
        assert_eq!(clean_block_string("   hi"), "   hi");
        assert_eq!(clean_block_string(r#"foo\"\"\"bar"#), r#"foo\"\"\"bar"#);
    }
}
