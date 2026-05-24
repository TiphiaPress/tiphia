use chrono::{DateTime, TimeZone, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QuerySelect, Set};
use std::collections::{HashMap, HashSet};
use tiphia_core::entities::{
    comments,
    comments::CommentStatus,
    options, post_terms, posts,
    posts::{PostStatus, PostType},
    terms::{self, TermType},
};

use crate::content::{
    non_empty, normalize_slug, render_markdown, strip_typecho_markers, trim_to_512, unique_slug,
};
use crate::typecho::{TypechoComment, TypechoContent, TypechoMeta};

pub async fn load_existing_term_slugs(db: &DatabaseConnection) -> anyhow::Result<HashSet<String>> {
    let rows = terms::Entity::find()
        .select_only()
        .column(terms::Column::Slug)
        .into_tuple::<String>()
        .all(db)
        .await?;
    Ok(rows.into_iter().map(normalize_slug).collect())
}

pub async fn load_existing_post_slugs(db: &DatabaseConnection) -> anyhow::Result<HashSet<String>> {
    let rows = posts::Entity::find()
        .select_only()
        .column(posts::Column::Slug)
        .into_tuple::<String>()
        .all(db)
        .await?;
    Ok(rows.into_iter().map(normalize_slug).collect())
}

pub async fn import_terms(
    db: &DatabaseConnection,
    metas: Vec<TypechoMeta>,
    used_term_slugs: &mut HashSet<String>,
) -> anyhow::Result<HashMap<u64, i32>> {
    let mut term_id_by_mid = HashMap::new();

    for meta in metas {
        let term_type = match meta.meta_type.as_str() {
            "category" => TermType::Category,
            "tag" => TermType::Tag,
            _ => continue,
        };
        let now = Utc::now();
        let slug = unique_slug(
            used_term_slugs,
            non_empty(meta.slug, format!("typecho-term-{}", meta.mid)),
        );
        let term = terms::ActiveModel {
            slug: Set(slug),
            name: Set(meta.name),
            description: Set(None),
            term_type: Set(term_type),
            parent_id: Set(None),
            sort_order: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        term_id_by_mid.insert(meta.mid, term.id);
    }

    Ok(term_id_by_mid)
}

pub async fn import_posts(
    db: &DatabaseConnection,
    contents: Vec<TypechoContent>,
    used_post_slugs: &mut HashSet<String>,
    author_id: i32,
) -> anyhow::Result<HashMap<u64, i32>> {
    let mut post_id_by_cid = HashMap::new();

    for content in contents {
        let post_type = match content.content_type.as_str() {
            "page" => PostType::Page,
            _ => PostType::Post,
        };
        let status = match content.status.as_str() {
            "publish" => PostStatus::Published,
            _ => PostStatus::Draft,
        };
        let markdown = strip_typecho_markers(&content.text);
        let html = render_markdown(&markdown);
        let created_at = timestamp(content.created);
        let updated_at = timestamp(content.modified);
        let slug = unique_slug(
            used_post_slugs,
            non_empty(content.slug, format!("typecho-post-{}", content.cid)),
        );
        let post = posts::ActiveModel {
            slug: Set(slug),
            title: Set(content.title),
            markdown: Set(markdown),
            html: Set(html),
            excerpt: Set(None),
            status: Set(status),
            post_type: Set(post_type),
            author_id: Set(author_id),
            published_at: Set(Some(created_at)),
            created_at: Set(created_at),
            updated_at: Set(updated_at),
            ..Default::default()
        }
        .insert(db)
        .await?;
        import_view_count(db, post.id, content.views).await?;
        post_id_by_cid.insert(content.cid, post.id);
    }

    Ok(post_id_by_cid)
}

pub async fn import_relationships(
    db: &DatabaseConnection,
    relations: Vec<(u64, u64)>,
    post_id_by_cid: &HashMap<u64, i32>,
    term_id_by_mid: &HashMap<u64, i32>,
) -> anyhow::Result<()> {
    for (cid, mid) in relations {
        let Some(post_id) = post_id_by_cid.get(&cid).copied() else {
            continue;
        };
        let Some(term_id) = term_id_by_mid.get(&mid).copied() else {
            continue;
        };
        post_terms::ActiveModel {
            post_id: Set(post_id),
            term_id: Set(term_id),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub async fn import_comments(
    db: &DatabaseConnection,
    source_comments: Vec<TypechoComment>,
    post_id_by_cid: &HashMap<u64, i32>,
) -> anyhow::Result<()> {
    let mut comment_id_by_coid = HashMap::new();

    for source in source_comments {
        let Some(post_id) = post_id_by_cid.get(&source.cid).copied() else {
            continue;
        };
        let parent_id = source
            .parent
            .and_then(|coid| comment_id_by_coid.get(&coid).copied());
        let created_at = timestamp(source.created);
        let comment = comments::ActiveModel {
            post_id: Set(post_id),
            parent_id: Set(parent_id),
            author_name: Set(non_empty(source.author, "Anonymous".to_owned())),
            author_email: Set(non_empty(
                source.mail,
                "anonymous@example.invalid".to_owned(),
            )),
            author_url: Set(source.url),
            ip_hash: Set(source.ip.map(|ip| format!("typecho:{ip}"))),
            user_agent: Set(source.agent.map(trim_to_512)),
            content: Set(source.text),
            status: Set(typecho_comment_status(&source.status)),
            created_at: Set(created_at),
            updated_at: Set(created_at),
            ..Default::default()
        }
        .insert(db)
        .await?;
        comment_id_by_coid.insert(source.coid, comment.id);
    }

    Ok(())
}

async fn import_view_count(
    db: &DatabaseConnection,
    post_id: i32,
    views: u64,
) -> anyhow::Result<()> {
    if views == 0 {
        return Ok(());
    }
    let now = Utc::now();
    options::ActiveModel {
        key: Set(format!("post:view:{post_id}")),
        value: Set(serde_json::json!({ "count": views })),
        autoload: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

fn typecho_comment_status(status: &str) -> CommentStatus {
    match status {
        "approved" => CommentStatus::Approved,
        "spam" => CommentStatus::Spam,
        "trash" => CommentStatus::Trash,
        _ => CommentStatus::Pending,
    }
}

fn timestamp(value: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(value, 0)
        .single()
        .unwrap_or_else(Utc::now)
}
