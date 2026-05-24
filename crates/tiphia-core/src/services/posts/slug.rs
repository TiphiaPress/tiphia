use crate::{
    app::AppState,
    entities::posts::{self, PostType},
    error::AppResult,
    services::validation,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub async fn normalize_create_slug(
    state: &AppState,
    raw_slug: &str,
    title: &str,
    post_type: &PostType,
) -> AppResult<String> {
    let slug = raw_slug.trim();
    if !slug.is_empty() {
        validation::slug(slug)?;
        return Ok(slug.to_owned());
    }

    let base = slugify(title).unwrap_or_else(|| fallback_slug(post_type).to_owned());
    unique_slug(state, &base).await
}

fn fallback_slug(post_type: &PostType) -> &'static str {
    match post_type {
        PostType::Post => "post",
        PostType::Page => "page",
    }
}

async fn unique_slug(state: &AppState, base: &str) -> AppResult<String> {
    let mut candidate = base.to_owned();
    let mut suffix = 2;

    loop {
        let exists = posts::Entity::find()
            .filter(posts::Column::Slug.eq(candidate.clone()))
            .one(&state.db)
            .await?
            .is_some();
        if !exists {
            return Ok(candidate);
        }

        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
}

fn slugify(value: &str) -> Option<String> {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() { None } else { Some(slug) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_normalizes_ascii_titles() {
        assert_eq!(
            slugify(" Hello, Rust 2026! ").as_deref(),
            Some("hello-rust-2026")
        );
        assert_eq!(slugify("A---B___C").as_deref(), Some("a-b-c"));
    }

    #[test]
    fn slugify_returns_none_for_non_ascii_titles() {
        assert_eq!(slugify("你好世界"), None);
        assert_eq!(slugify("  ---  "), None);
    }
}
