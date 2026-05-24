use crate::{
    app::AppState,
    entities::posts::PostType,
    pagination::PaginationQuery,
    services::posts::{ListPostQuery, PostResponse},
};

pub async fn public_items(
    state: &AppState,
    post_type: Option<PostType>,
    per_page: u64,
) -> crate::AppResult<Vec<PostResponse>> {
    let page = crate::services::posts::list(
        state,
        ListPostQuery {
            q: None,
            status: None,
            post_type,
            term_id: None,
            pagination: PaginationQuery {
                page: Some(1),
                per_page: Some(per_page),
            },
        },
    )
    .await?;

    Ok(page.data)
}
