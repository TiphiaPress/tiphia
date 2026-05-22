use chrono::Utc;
use criterion::{Criterion, criterion_group, criterion_main};
use tiphia_core::{
    entities::comments::{self, CommentStatus},
    services::comments::build_comment_tree_for_bench,
};

fn comment(id: i32, parent_id: Option<i32>) -> comments::Model {
    comments::Model {
        id,
        post_id: 1,
        parent_id,
        author_name: "Bench".to_owned(),
        author_email: "bench@example.com".to_owned(),
        author_url: None,
        ip_hash: None,
        user_agent: None,
        content: "content".to_owned(),
        status: CommentStatus::Approved,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn bench_comment_tree(c: &mut Criterion) {
    let comments = (1..=1_000)
        .map(|id| {
            let parent_id = if id > 1 && id % 3 != 0 {
                Some((id - 1).max(1))
            } else {
                None
            };
            comment(id, parent_id)
        })
        .collect::<Vec<_>>();

    c.bench_function("comment_tree_1000", |b| {
        b.iter(|| build_comment_tree_for_bench(comments.clone()))
    });
}

criterion_group!(benches, bench_comment_tree);
criterion_main!(benches);
