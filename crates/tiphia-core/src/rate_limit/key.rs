use axum::http::{HeaderMap, header};

pub fn client_key(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .or_else(|| headers.get(header::USER_AGENT))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn client_key_prefers_first_forwarded_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("1.1.1.1, 2.2.2.2"),
        );
        assert_eq!(client_key(&headers), "1.1.1.1");
    }

    #[test]
    fn client_key_falls_back_to_unknown() {
        assert_eq!(client_key(&HeaderMap::new()), "unknown");
    }
}
