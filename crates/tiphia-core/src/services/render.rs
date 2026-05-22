use crate::{
    app::AppState,
    error::{AppError, AppResult},
    plugins::{Hook, HookContext},
};
use ammonia::Builder;
use pulldown_cmark::{Options, Parser, html};
use serde::{Deserialize, Serialize};

const DEFAULT_EXCERPT_LEN: usize = 220;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderInput {
    pub markdown: String,
    pub html: Option<String>,
    pub excerpt: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderedContent {
    pub markdown: String,
    pub html: String,
    pub excerpt: String,
}

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
        .unwrap_or_else(|| markdown_to_html(&input.markdown));
    let sanitized_html = sanitize_html(&raw_html);
    let excerpt = input
        .excerpt
        .unwrap_or_else(|| excerpt_from_markdown(&input.markdown, DEFAULT_EXCERPT_LEN));
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

fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

fn sanitize_html(raw_html: &str) -> String {
    Builder::default()
        .add_tags(["table", "thead", "tbody", "tr", "th", "td"])
        .add_generic_attributes(["class"])
        .clean(raw_html)
        .to_string()
}

fn excerpt_from_markdown(markdown: &str, max_chars: usize) -> String {
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
