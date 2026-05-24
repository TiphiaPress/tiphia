use crate::{entities::posts::PostType, plugins::Hook};

use super::ListPostQuery;

pub fn before_list(query: &ListPostQuery) -> Hook {
    match query.post_type {
        Some(PostType::Page) => Hook::BeforePageList,
        _ => Hook::BeforePostList,
    }
}

pub fn after_list(query: &ListPostQuery) -> Hook {
    match query.post_type {
        Some(PostType::Page) => Hook::AfterPageList,
        _ => Hook::AfterPostList,
    }
}

pub fn before_create(post_type: &PostType) -> Hook {
    match post_type {
        PostType::Page => Hook::BeforePageCreate,
        PostType::Post => Hook::BeforePostCreate,
    }
}

pub fn after_create(post_type: &PostType) -> Hook {
    match post_type {
        PostType::Page => Hook::AfterPageCreate,
        PostType::Post => Hook::AfterPostCreate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pagination::PaginationQuery;

    #[test]
    fn list_hooks_follow_query_post_type() {
        let page_query = ListPostQuery {
            q: None,
            status: None,
            post_type: Some(PostType::Page),
            term_id: None,
            pagination: PaginationQuery::default(),
        };
        let post_query = ListPostQuery {
            post_type: Some(PostType::Post),
            ..page_query.clone()
        };

        assert!(matches!(before_list(&page_query), Hook::BeforePageList));
        assert!(matches!(after_list(&page_query), Hook::AfterPageList));
        assert!(matches!(before_list(&post_query), Hook::BeforePostList));
        assert!(matches!(after_list(&post_query), Hook::AfterPostList));
    }

    #[test]
    fn create_hooks_follow_post_type() {
        assert!(matches!(
            before_create(&PostType::Page),
            Hook::BeforePageCreate
        ));
        assert!(matches!(
            after_create(&PostType::Page),
            Hook::AfterPageCreate
        ));
        assert!(matches!(
            before_create(&PostType::Post),
            Hook::BeforePostCreate
        ));
        assert!(matches!(
            after_create(&PostType::Post),
            Hook::AfterPostCreate
        ));
    }
}
