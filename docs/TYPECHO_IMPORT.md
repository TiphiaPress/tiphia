# Typecho Import Tool

`tiphia-typecho-import` migrates Typecho posts, pages, categories, tags, and
comments into a Tiphia database.

It reads a Typecho MySQL database and writes to the Tiphia database through
SeaORM. It also runs core migrations before import.

## Usage

Dry run:

```bash
cargo run -p tiphia-typecho-import -- \
  --typecho-url "mysql://user:password@127.0.0.1:3306/typecho" \
  --typecho-prefix "typecho_" \
  --tiphia-url "sqlite://tiphia.db?mode=rwc" \
  --author-id 1
```

Execute import:

```bash
cargo run -p tiphia-typecho-import -- \
  --typecho-url "mysql://user:password@127.0.0.1:3306/typecho" \
  --typecho-prefix "typecho_" \
  --tiphia-url "sqlite://tiphia.db?mode=rwc" \
  --author-id 1 \
  --execute
```

## Imported Data

- Typecho `post` rows become Tiphia articles.
- Typecho `page` rows become Tiphia pages.
- Typecho categories and tags become Tiphia terms.
- Typecho relationships become article-term links.
- Typecho comments become Tiphia comments, including nested parent comments when
  the parent comment is imported.
- Markdown is rendered to sanitized HTML during import.
- Typecho view counts are imported into the Tiphia `options` table using `post:view:{post_id}` with value `{ "count": number }`.

Run the dry run first and back up both databases before executing. The importer
does not delete existing Tiphia data.

## View Count Contract

Tiphia stores post view counts in the `options` table instead of the `posts` table.
The importer must use the same key as the core runtime:

```text
post:view:{post_id}
```

Value example:

```json
{
  "count": 1234
}
```

Do not create alternative keys such as `views:{id}` or `post_views:{id}`. The default theme, popular-post widgets, and future statistics tools all depend on the `post:view:{post_id}` contract.