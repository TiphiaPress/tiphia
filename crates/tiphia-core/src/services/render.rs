use crate::{
    app::AppState,
    error::{AppError, AppResult},
    plugins::{Hook, HookContext},
};

#[path = "render/excerpt.rs"]
mod excerpt;
#[path = "render/markdown.rs"]
mod markdown;
#[path = "render/model.rs"]
mod model;
#[path = "render/sanitize.rs"]
mod sanitize;

pub use model::{RenderInput, RenderedContent};

pub async fn render_content(state: &AppState, input: RenderInput) -> AppResult<RenderedContent> {
    let mut context = HookContext::with_subject(input)?;
    state
        .plugins
        .dispatch(Hook::BeforeRender, &mut context)
        .await?;
    context.ensure_not_stopped()?;

    let input = context
        .take_subject::<RenderInput>()?
        .ok_or_else(|| AppError::Plugin("render hook removed render input".to_owned()))?;
    let raw_html = input
        .html
        .unwrap_or_else(|| markdown::markdown_to_html(&input.markdown));
    let sanitized_html = sanitize::sanitize_html(&raw_html);
    let excerpt = input.excerpt.unwrap_or_else(|| {
        excerpt::excerpt_from_markdown(&input.markdown, excerpt::DEFAULT_EXCERPT_LEN)
    });
    let mut rendered = RenderedContent {
        markdown: input.markdown,
        html: sanitized_html,
        excerpt,
    };

    let mut context = HookContext::with_subject(&rendered)?;
    state
        .plugins
        .dispatch(Hook::AfterRender, &mut context)
        .await?;
    context.ensure_not_stopped()?;
    if let Some(next_rendered) = context.take_subject::<RenderedContent>()? {
        rendered = next_rendered;
    }

    Ok(rendered)
}
