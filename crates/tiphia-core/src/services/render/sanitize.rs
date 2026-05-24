use ammonia::Builder;

pub fn sanitize_html(raw_html: &str) -> String {
    Builder::default()
        .add_tags(["table", "thead", "tbody", "tr", "th", "td"])
        .add_generic_attributes(["class"])
        .clean(raw_html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_removes_script_but_keeps_tables() {
        let html = sanitize_html("<script>alert(1)</script><table><tr><td>x</td></tr></table>");
        assert!(!html.contains("script"));
        assert!(html.contains("<table>"));
    }
}
