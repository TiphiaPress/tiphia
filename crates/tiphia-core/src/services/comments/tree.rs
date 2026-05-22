use crate::entities::comments;
use serde::Serialize;
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct CommentNode {
    #[serde(flatten)]
    pub comment: comments::Model,
    #[schema(no_recursion)]
    pub children: Vec<CommentNode>,
}

pub fn build(comments: Vec<comments::Model>) -> Vec<CommentNode> {
    let mut by_parent = comments.into_iter().fold(
        HashMap::<Option<i32>, Vec<comments::Model>>::new(),
        |mut by_parent, comment| {
            by_parent
                .entry(comment.parent_id)
                .or_default()
                .push(comment);
            by_parent
        },
    );

    for siblings in by_parent.values_mut() {
        siblings.sort_by_key(|comment| comment.created_at);
    }

    build_branch(&mut by_parent, None)
}

fn build_branch(
    by_parent: &mut HashMap<Option<i32>, Vec<comments::Model>>,
    parent_id: Option<i32>,
) -> Vec<CommentNode> {
    by_parent
        .remove(&parent_id)
        .unwrap_or_default()
        .into_iter()
        .map(|comment| {
            let children = build_branch(by_parent, Some(comment.id));
            CommentNode { comment, children }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::comments::CommentStatus;
    use chrono::Utc;

    fn comment(id: i32, parent_id: Option<i32>) -> comments::Model {
        comments::Model {
            id,
            post_id: 1,
            parent_id,
            author_name: "Alice".to_owned(),
            author_email: "alice@example.com".to_owned(),
            author_url: None,
            ip_hash: None,
            user_agent: None,
            content: "hello".to_owned(),
            status: CommentStatus::Approved,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn builds_nested_comment_tree() {
        let tree = build(vec![
            comment(1, None),
            comment(2, Some(1)),
            comment(3, Some(2)),
            comment(4, None),
        ]);

        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].children.len(), 1);
        assert_eq!(tree[1].comment.id, 4);
    }

    #[test]
    fn preserves_sibling_order_by_created_at() {
        let mut newer = comment(1, None);
        newer.created_at = Utc::now();
        let mut older = comment(2, None);
        older.created_at = newer.created_at - chrono::Duration::seconds(10);

        let tree = build(vec![newer, older]);

        assert_eq!(tree[0].comment.id, 2);
        assert_eq!(tree[1].comment.id, 1);
    }
}
