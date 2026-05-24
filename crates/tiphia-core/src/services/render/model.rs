use serde::{Deserialize, Serialize};

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
