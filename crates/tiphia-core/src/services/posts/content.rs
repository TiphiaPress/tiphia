use crate::{
    app::AppState,
    entities::posts,
    error::AppResult,
    services::{
        posts::{CreatePostInput, UpdatePostInput},
        render::{RenderInput, RenderedContent, render_content},
    },
};

pub async fn render_create(
    state: &AppState,
    input: &CreatePostInput,
) -> AppResult<RenderedContent> {
    render_content(
        state,
        RenderInput {
            markdown: input.markdown.clone(),
            html: input.html.clone(),
            excerpt: input.excerpt.clone(),
        },
    )
    .await
}

pub async fn render_update(
    state: &AppState,
    existing: &posts::Model,
    input: &UpdatePostInput,
) -> AppResult<Option<RenderedContent>> {
    if !needs_render(input) {
        return Ok(None);
    }

    let markdown = input
        .markdown
        .clone()
        .unwrap_or_else(|| existing.markdown.clone());

    render_content(
        state,
        RenderInput {
            markdown,
            html: input.html.clone(),
            excerpt: input.excerpt.clone(),
        },
    )
    .await
    .map(Some)
}

fn needs_render(input: &UpdatePostInput) -> bool {
    input.markdown.is_some() || input.html.is_some() || input.excerpt.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_needs_render_when_content_fields_change() {
        assert!(!needs_render(&UpdatePostInput {
            slug: Some("slug".to_owned()),
            title: Some("Title".to_owned()),
            markdown: None,
            html: None,
            excerpt: None,
            status: None,
            published_at: None,
        }));

        assert!(needs_render(&UpdatePostInput {
            slug: None,
            title: None,
            markdown: Some("body".to_owned()),
            html: None,
            excerpt: None,
            status: None,
            published_at: None,
        }));
    }
}
