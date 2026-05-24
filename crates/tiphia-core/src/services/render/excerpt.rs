pub const DEFAULT_EXCERPT_LEN: usize = 220;

pub fn excerpt_from_markdown(markdown: &str, max_chars: usize) -> String {
    let mut plain = String::with_capacity(markdown.len());
    let mut in_code_block = false;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        let line = trimmed
            .trim_start_matches('#')
            .trim_start_matches('>')
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim();
        if !line.is_empty() {
            if !plain.is_empty() {
                plain.push(' ');
            }
            plain.push_str(line);
        }
    }

    let mut excerpt = plain.chars().take(max_chars).collect::<String>();
    if plain.chars().count() > max_chars {
        excerpt.push_str("...");
    }
    excerpt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excerpt_skips_fenced_code_blocks() {
        let excerpt = excerpt_from_markdown("# Title\n```rs\nsecret\n```\nBody", 220);
        assert_eq!(excerpt, "Title Body");
    }
}
