mod content;
mod target;
mod typecho;

use clap::Parser;
use sea_orm::Database;
use sqlx::MySqlPool;
use target::{
    import_comments, import_posts, import_relationships, import_terms, load_existing_post_slugs,
    load_existing_term_slugs,
};
use tiphia_core::migration::run_core_migrations;
use typecho::{load_comments, load_contents, load_metas, load_relationships};

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

    let mut used_term_slugs = load_existing_term_slugs(&tiphia).await?;
    let mut used_post_slugs = load_existing_post_slugs(&tiphia).await?;

    let term_id_by_mid = import_terms(&tiphia, metas, &mut used_term_slugs).await?;
    let post_id_by_cid =
        import_posts(&tiphia, contents, &mut used_post_slugs, args.author_id).await?;
    import_relationships(&tiphia, relations, &post_id_by_cid, &term_id_by_mid).await?;
    import_comments(&tiphia, comments, &post_id_by_cid).await?;

    println!("Import completed.");
    Ok(())
}
