use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Default)]
pub struct CommentRequestMeta {
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

pub fn hash_client_ip(secret: &str, ip: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b":comment-ip:");
    hasher.update(ip.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn normalize_user_agent(user_agent: String) -> Option<String> {
    let user_agent = user_agent.trim();
    if user_agent.is_empty() {
        return None;
    }

    Some(user_agent.chars().take(512).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_client_ip_with_secret() {
        let hash = hash_client_ip("secret", "203.0.113.10");

        assert_eq!(hash.len(), 64);
        assert_ne!(hash, hash_client_ip("other-secret", "203.0.113.10"));
        assert_ne!(hash, "203.0.113.10");
    }

    #[test]
    fn user_agent_is_trimmed_and_limited() {
        assert_eq!(
            normalize_user_agent("  Browser  ".to_owned()).unwrap(),
            "Browser"
        );
        assert!(normalize_user_agent(" ".to_owned()).is_none());
        assert_eq!(normalize_user_agent("a".repeat(600)).unwrap().len(), 512);
    }
}
