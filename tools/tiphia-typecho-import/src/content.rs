use ammonia::Builder;
use pulldown_cmark::{Options, Parser as MarkdownParser, html};
use std::collections::HashSet;

pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = MarkdownParser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    sanitize_html(&html_output)
}

pub fn strip_typecho_markers(text: &str) -> String {
    text.trim_start_matches("<!--markdown-->").trim().to_owned()
}

pub fn unique_slug(used: &mut HashSet<String>, raw: String) -> String {
    let base = normalize_slug(raw);
    let base = if base.is_empty() {
        "typecho-imported".to_owned()
    } else {
        base
    };
    if used.insert(base.clone()) {
        return base;
    }

    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded slug suffix loop should always return")
}

pub fn normalize_slug(value: String) -> String {
    let value = value.trim().trim_matches('/').to_lowercase();
    let mut out = String::with_capacity(value.len());
    let mut last_dash = false;

    for ch in value.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            Some(ch)
        } else if ch.is_whitespace() || matches!(ch, '/' | '\\' | '.' | ',' | ':' | ';') {
            Some('-')
        } else if !ch.is_control() {
            Some(ch)
        } else {
            None
        };

        if let Some(ch) = mapped {
            if ch == '-' {
                if !last_dash {
                    out.push(ch);
                }
                last_dash = true;
            } else {
                out.push(ch);
                last_dash = false;
            }
        }
    }

    out.trim_matches('-').to_owned()
}

pub fn non_empty(value: String, fallback: String) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback
    } else {
        value.to_owned()
    }
}

pub fn trim_to_512(value: String) -> String {
    value.trim().chars().take(512).collect()
}

fn sanitize_html(raw_html: &str) -> String {
    Builder::default()
        .add_tags(["table", "thead", "tbody", "tr", "th", "td"])
        .add_generic_attributes(["class"])
        .clean(raw_html)
        .to_string()
}
