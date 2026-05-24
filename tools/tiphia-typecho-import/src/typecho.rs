use anyhow::{Context, bail};
use sqlx::{MySqlPool, Row, mysql::MySqlRow};

#[derive(Debug)]
pub struct TypechoContent {
    pub cid: u64,
    pub title: String,
    pub slug: String,
    pub text: String,
    pub created: i64,
    pub modified: i64,
    pub content_type: String,
    pub status: String,
    pub views: u64,
}

#[derive(Debug)]
pub struct TypechoMeta {
    pub mid: u64,
    pub name: String,
    pub slug: String,
    pub meta_type: String,
}

#[derive(Debug)]
pub struct TypechoComment {
    pub coid: u64,
    pub cid: u64,
    pub parent: Option<u64>,
    pub author: String,
    pub mail: String,
    pub url: Option<String>,
    pub ip: Option<String>,
    pub agent: Option<String>,
    pub text: String,
    pub created: i64,
    pub status: String,
}

pub async fn load_contents(pool: &MySqlPool, prefix: &str) -> anyhow::Result<Vec<TypechoContent>> {
    let sql = format!(
        "select cid, title, slug, text, created, modified, type, status, views from `{prefix}contents` where type in ('post', 'page')"
    );
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(TypechoContent {
                cid: mysql_unsigned(&row, "cid")?,
                title: mysql_string(&row, "title")?,
                slug: mysql_string(&row, "slug")?,
                text: mysql_string(&row, "text")?,
                created: mysql_timestamp(&row, "created")?,
                modified: mysql_timestamp(&row, "modified")?,
                content_type: mysql_string(&row, "type")?,
                status: mysql_string(&row, "status")?,
                views: mysql_unsigned(&row, "views").unwrap_or(0),
            })
        })
        .collect()
}

pub async fn load_metas(pool: &MySqlPool, prefix: &str) -> anyhow::Result<Vec<TypechoMeta>> {
    let sql = format!(
        "select mid, name, slug, type from `{prefix}metas` where type in ('category', 'tag')"
    );
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(TypechoMeta {
                mid: mysql_unsigned(&row, "mid")?,
                name: mysql_string(&row, "name")?,
                slug: mysql_string(&row, "slug")?,
                meta_type: mysql_string(&row, "type")?,
            })
        })
        .collect()
}

pub async fn load_relationships(pool: &MySqlPool, prefix: &str) -> anyhow::Result<Vec<(u64, u64)>> {
    let sql = format!("select cid, mid from `{prefix}relationships`");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| Ok((mysql_unsigned(&row, "cid")?, mysql_unsigned(&row, "mid")?)))
        .collect()
}

pub async fn load_comments(pool: &MySqlPool, prefix: &str) -> anyhow::Result<Vec<TypechoComment>> {
    let sql = format!(
        "select coid, cid, parent, author, mail, url, ip, agent, text, created, status from `{prefix}comments` order by coid asc"
    );
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            let parent = mysql_unsigned(&row, "parent")?;
            Ok(TypechoComment {
                coid: mysql_unsigned(&row, "coid")?,
                cid: mysql_unsigned(&row, "cid")?,
                parent: (parent > 0).then_some(parent),
                author: mysql_string_or_default(&row, "author", "Anonymous")?,
                mail: mysql_string_or_default(&row, "mail", "anonymous@example.invalid")?,
                url: mysql_optional_string(&row, "url")?,
                ip: mysql_optional_string(&row, "ip")?,
                agent: mysql_optional_string(&row, "agent")?,
                text: mysql_string_or_default(&row, "text", "")?,
                created: mysql_timestamp(&row, "created")?,
                status: mysql_string_or_default(&row, "status", "waiting")?,
            })
        })
        .collect()
}

fn mysql_unsigned(row: &MySqlRow, column: &'static str) -> anyhow::Result<u64> {
    if let Ok(value) = row.try_get::<u64, _>(column) {
        return Ok(value);
    }
    if let Ok(value) = row.try_get::<u32, _>(column) {
        return Ok(u64::from(value));
    }
    if let Ok(value) = row.try_get::<i64, _>(column) {
        if value >= 0 {
            return Ok(value as u64);
        }
        bail!("column `{column}` contains negative id {value}");
    }
    if let Ok(value) = row.try_get::<i32, _>(column) {
        if value >= 0 {
            return Ok(value as u64);
        }
        bail!("column `{column}` contains negative id {value}");
    }
    row.try_get::<u32, _>(column)
        .map(u64::from)
        .with_context(|| format!("failed to decode unsigned integer column `{column}`"))
}

fn mysql_timestamp(row: &MySqlRow, column: &'static str) -> anyhow::Result<i64> {
    mysql_unsigned(row, column).and_then(|value| {
        i64::try_from(value).with_context(|| format!("timestamp column `{column}` is too large"))
    })
}

fn mysql_string(row: &MySqlRow, column: &'static str) -> anyhow::Result<String> {
    row.try_get::<String, _>(column)
        .with_context(|| format!("failed to decode string column `{column}`"))
}

fn mysql_string_or_default(
    row: &MySqlRow,
    column: &'static str,
    fallback: &str,
) -> anyhow::Result<String> {
    Ok(mysql_optional_string(row, column)?.unwrap_or_else(|| fallback.to_owned()))
}

fn mysql_optional_string(row: &MySqlRow, column: &'static str) -> anyhow::Result<Option<String>> {
    let value = row
        .try_get::<Option<String>, _>(column)
        .with_context(|| format!("failed to decode optional string column `{column}`"))?;
    Ok(value.and_then(empty_to_none))
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}
