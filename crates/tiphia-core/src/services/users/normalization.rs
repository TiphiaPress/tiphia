pub fn normalize_email(value: String) -> String {
    value.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_is_trimmed_and_lowercased() {
        assert_eq!(
            normalize_email("  USER@Example.COM  ".to_owned()),
            "user@example.com"
        );
    }
}
