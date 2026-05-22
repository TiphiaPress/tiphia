use ammonia::Builder;
use chrono::{DateTime, TimeZone, Utc};
use clap::Parser;
use pulldown_cmark::{Options, Parser as MarkdownParser, html};
use sea_orm::{ActiveModelTrait, Database, Set};
use sqlx::{MySqlPool, Row};
use std::collections::HashMap;
use tiphia_core::{
    entities::{
        comments,
        comments::CommentStatus,
        post_terms, posts,
        posts::{PostStatus, PostType},
        terms::{self, TermType},
    },
    migration::run_core_migrations,
};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    typecho_url: String,
    #[arg(long, default_value = "typecho_")]
    typecho_prefix: String,
    #[arg(long)]
    tiphia_url: String,
    #[arg(long)]
    author_id: i32,
    #[arg(long)]
    execute: bool,
}

#[derive(Debug)]
struct TypechoContent {
    cid: i32,
    title: String,
    slug: String,
    text: String,
    created: i64,
    modified: i64,
    content_type: String,
    status: String,
}

#[derive(Debug)]
struct TypechoMeta {
    mid: i32,
    name: String,
    slug: String,
    meta_type: String,
}

#[derive(Debug)]
struct TypechoComment {
    coid: i32,
    cid: i32,
    parent: Option<i32>,
    author: String,
    mail: String,
    url: Option<String>,
    ip: Option<String>,
    agent: Option<String>,
    text: String,
    created: i64,
    status: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let typecho = MySqlPool::connect(&args.typecho_url).await?;
    let tiphia = Database::connect(&args.tiphia_url).await?;
    run_core_migrations(&tiphia).await?;

    let contents = load_contents(&typecho, &args.typecho_prefix).await?;
    let metas = load_metas(&typecho, &args.typecho_prefix).await?;
    let relations = load_relationships(&typecho, &args.typecho_prefix).await?;
    let comments = load_comments(&typecho, &args.typecho_prefix).await?;

    println!(
        "Found {} posts/pages, {} metas, {} relationships, {} comments",
        contents.len(),
        metas.len(),
        relations.len(),
        comments.len()
    );

    if !args.execute {
        println!("Dry run only. Re-run with --execute to import.");
        return Ok(());
    }

    let mut term_id_by_mid = HashMap::new();
    for meta in metas {
        let term_type = match meta.meta_type.as_str() {
            "category" => TermType::Category,
            "tag" => TermType::Tag,
            _ => continue,
        };
        let now = Utc::now();
        let term = terms::ActiveModel {
            slug: Set(non_empty(meta.slug, format!("typecho-term-{}", meta.mid))),
            name: Set(meta.name),
            description: Set(None),
            term_type: Set(term_type),
            parent_id: Set(None),
            sort_order: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tiphia)
        .await?;
        term_id_by_mid.insert(meta.mid, term.id);
    }

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
        let post = posts::ActiveModel {
            slug: Set(non_empty(
                content.slug,
                format!("typecho-post-{}", content.cid),
            )),
            title: Set(content.title),
            markdown: Set(markdown),
            html: Set(html),
            excerpt: Set(None),
            status: Set(status),
            post_type: Set(post_type),
            author_id: Set(args.author_id),
            published_at: Set(Some(created_at)),
            created_at: Set(created_at),
            updated_at: Set(updated_at),
            ..Default::default()
        }
        .insert(&tiphia)
        .await?;
        post_id_by_cid.insert(content.cid, post.id);
    }

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
        .insert(&tiphia)
        .await?;
    }

    import_comments(&tiphia, comments, &post_id_by_cid).await?;

    println!("Import completed.");
    Ok(())
}

async fn load_contents(pool: &MySqlPool, prefix: &str) -> anyhow::Result<Vec<TypechoContent>> {
    let sql = format!(
        "select cid, title, slug, text, created, modified, type, status from `{prefix}contents` where type in ('post', 'page')"
    );
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| TypechoContent {
            cid: row.get("cid"),
            title: row.get("title"),
            slug: row.get("slug"),
            text: row.get("text"),
            created: row.get::<i32, _>("created") as i64,
            modified: row.get::<i32, _>("modified") as i64,
            content_type: row.get("type"),
            status: row.get("status"),
        })
        .collect())
}

async fn load_metas(pool: &MySqlPool, prefix: &str) -> anyhow::Result<Vec<TypechoMeta>> {
    let sql = format!(
        "select mid, name, slug, type from `{prefix}metas` where type in ('category', 'tag')"
    );
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| TypechoMeta {
            mid: row.get("mid"),
            name: row.get("name"),
            slug: row.get("slug"),
            meta_type: row.get("type"),
        })
        .collect())
}

async fn load_relationships(pool: &MySqlPool, prefix: &str) -> anyhow::Result<Vec<(i32, i32)>> {
    let sql = format!("select cid, mid from `{prefix}relationships`");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("cid"), row.get("mid")))
        .collect())
}

async fn load_comments(pool: &MySqlPool, prefix: &str) -> anyhow::Result<Vec<TypechoComment>> {
    let sql = format!(
        "select coid, cid, parent, author, mail, url, ip, agent, text, created, status from `{prefix}comments` order by coid asc"
    );
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let parent = row.get::<i32, _>("parent");
            TypechoComment {
                coid: row.get("coid"),
                cid: row.get("cid"),
                parent: (parent > 0).then_some(parent),
                author: row.get("author"),
                mail: row.get("mail"),
                url: empty_to_none(row.get("url")),
                ip: empty_to_none(row.get("ip")),
                agent: empty_to_none(row.get("agent")),
                text: row.get("text"),
                created: row.get::<i32, _>("created") as i64,
                status: row.get("status"),
            }
        })
        .collect())
}

async fn import_comments(
    db: &sea_orm::DatabaseConnection,
    source_comments: Vec<TypechoComment>,
    post_id_by_cid: &HashMap<i32, i32>,
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

fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = MarkdownParser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    Builder::default().clean(&html_output).to_string()
}

fn strip_typecho_markers(text: &str) -> String {
    text.trim_start_matches("<!--markdown-->").trim().to_owned()
}

fn non_empty(value: String, fallback: String) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback
    } else {
        value.to_owned()
    }
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn trim_to_512(value: String) -> String {
    value.trim().chars().take(512).collect()
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
