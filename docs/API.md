# Tiphia REST API

Base path: `/api/v1`.

OpenAPI document:

```http
GET /openapi.json
```

The OpenAPI document is generated with `utoipa` from route annotations and Rust
schemas.

## Authentication

Authentication status:

```http
GET /api/v1/auth/status
```

Response:

```json
{
  "initialized": true,
  "registration_enabled": false
}
```

Create the first root administrator:

```http
POST /api/v1/auth/bootstrap
Content-Type: application/json

{
  "username": "admin",
  "email": "admin@example.com",
  "password": "change-me-please",
  "display_name": "Administrator"
}
```

This endpoint only works while the user table is empty.

Public registration is available only when site settings enable it:

```http
POST /api/v1/auth/register
Content-Type: application/json

{
  "username": "reader",
  "email": "reader@example.com",
  "password": "change-me-please",
  "display_name": "Reader"
}
```

Registered public users are created as `author`.

Login:

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "account": "admin",
  "password": "change-me-please"
}
```

Use the returned token for protected writes:

```http
Authorization: Bearer <access_token>
```

Current user:

```http
GET /api/v1/auth/me
Authorization: Bearer <access_token>
```

## Protected Writes

These endpoints require a bearer token:

- `POST /api/v1/posts`
- `PUT /api/v1/posts/{id}`
- `DELETE /api/v1/posts/{id}`
- `PUT /api/v1/posts/{id}/terms`
- `POST /api/v1/pages`
- `PUT /api/v1/pages/{id}`
- `DELETE /api/v1/pages/{id}`
- `PUT /api/v1/pages/{id}/terms`
- `POST /api/v1/terms`
- `PUT /api/v1/terms/{id}`
- `DELETE /api/v1/terms/{id}`
- `PUT /api/v1/comments/{id}/moderation`
- `GET /api/v1/users`
- `POST /api/v1/users`
- `GET /api/v1/users/{id}`
- `PUT /api/v1/users/{id}`
- `PUT /api/v1/users/{id}/password`
- `GET /api/v1/plugins/admin-menu`
- `GET /api/v1/plugins/{name}/config`
- `PUT /api/v1/plugins/{name}/config`
- `PUT /api/v1/settings`

Public reads and public comment creation remain open by default.

## Roles

- `Root`: highest administrator, can create and manage all roles.
- `Admin`: can manage taxonomy, comments, all content, editors, and authors. Admins cannot manage root users or peer admins.
- `Editor`: can manage taxonomy, comments, and all content.
- `Author`: can create content and update/delete only their own content.

Root and admin users cannot disable themselves.

User management is admin-only:

```http
GET /api/v1/users
Authorization: Bearer <access_token>
```

```http
POST /api/v1/users
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "username": "writer",
  "email": "writer@example.com",
  "password": "change-me-please",
  "display_name": "Writer",
  "role": "author"
}
```

Update a user:

```http
PUT /api/v1/users/2
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "email": "writer@example.com",
  "display_name": "Writer",
  "role": "author",
  "status": "active"
}
```

Disable a user by setting:

```json
{ "status": "disabled" }
```

Disabled users cannot log in.

Change a user's password:

```http
PUT /api/v1/users/2/password
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "password": "new-change-me-please"
}
```

Plugin configuration is admin-only:

```http
GET /api/v1/plugins/tiphia-audit/config
Authorization: Bearer <access_token>
```

```http
PUT /api/v1/plugins/tiphia-audit/config
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "config": {
    "log_post_events": true,
    "log_comment_events": true,
    "blocked_comment_words": ["spam", "casino"]
  }
}
```

Audit events are available to editors and admins:

```http
GET /api/v1/audit/events?page=1&per_page=20
Authorization: Bearer <access_token>
```

## Environment

- `TIPHIA_CONFIG`: config file path, default `tiphia.toml`.
- `TIPHIA_BIND`: HTTP bind address, default `127.0.0.1:3000`.
- `TIPHIA_REQUEST_TIMEOUT_SECS`: request timeout, default `30`.
- `TIPHIA_MAX_BODY_BYTES`: JSON/body extraction limit, default `1048576`.
- `DATABASE_URL`: database URL, default `sqlite://tiphia.db?mode=rwc`.
- `TIPHIA_DB_MAX_CONNECTIONS`: database pool maximum connections, default `16`.
- `TIPHIA_DB_MIN_CONNECTIONS`: database pool minimum connections, default `1`.
- `TIPHIA_DB_CONNECT_TIMEOUT_SECS`: database connect timeout, default `10`.
- `TIPHIA_DB_ACQUIRE_TIMEOUT_SECS`: database acquire timeout, default `10`.
- `TIPHIA_JWT_SECRET`: JWT signing secret. Must be changed in production.
- `TIPHIA_TOKEN_TTL_SECONDS`: token lifetime, default `604800`.
- `TIPHIA_CORS_ALLOWED_ORIGINS`: comma-separated allowed origins. Empty means permissive development CORS.
- `TIPHIA_RATE_LIMIT_LOGIN_PER_MINUTE`: login attempts per minute, default `5`.
- `TIPHIA_RATE_LIMIT_COMMENTS_PER_MINUTE`: public comment submissions per minute, default `10`.
- `TIPHIA_REDIS_URL`: Redis URL for distributed rate limiting. Empty uses in-memory fallback.

## Error Responses

Errors use a consistent JSON envelope:

```json
{
  "error": {
    "code": "validation_error",
    "message": "validation error: title is required"
  }
}
```

Common codes:

- `validation_error`: invalid input or uniqueness conflict, HTTP 422.
- `unauthorized`: missing or invalid token, HTTP 401.
- `forbidden`: authenticated but not allowed, HTTP 403.
- `not_found`: resource not found, HTTP 404.
- `rate_limited`: rate limit exceeded, HTTP 429.
- `rate_limit_backend`: Redis/rate-limit backend error, HTTP 503.
- `plugin_error`: plugin hook/config failure, HTTP 500.
- `internal_error`: unexpected server error, HTTP 500.

## Config File

Tiphia loads `tiphia.toml` by default. Use `TIPHIA_CONFIG` to point at another
file. Environment variables override file values, which is useful for secrets
and container deployments.

See `tiphia.example.toml` for a full example.

When `app.environment = "production"`, `auth.jwt_secret` must be changed from
the development default. Tiphia also validates bind address, CORS origins, Redis
URL format, and required database URL during startup.

## Database

Core and plugin schema changes are tracked in `schema_migrations`. Plugin
migrations should use globally unique ids such as:

```text
plugin:tiphia-audit:0001:create-audit-events
```

Each migration runs in its own transaction. Migrations may implement `down` for
rollback support. If a migration does not implement rollback, Tiphia reports that
the migration is not reversible instead of guessing a destructive operation.

## Rendering

Post and page writes run through the rendering pipeline:

- Markdown is rendered to HTML when `html` is omitted.
- Provided HTML is sanitized before storage.
- `excerpt` is generated from Markdown when omitted.
- `BeforeRender` and `AfterRender` plugin hooks can modify or stop rendering.

Supported Markdown extensions include tables, footnotes, strikethrough, and task
lists.

## Posts And Permalinks

Post and page responses include `permalink`, generated from
`settings.permalink_format`.

Supported permalink tokens:

- `{id}`
- `{slug}`
- `{year}`
- `{month}`
- `{day}`

Find content by slug:

```http
GET /api/v1/posts/slug/hello-world
GET /api/v1/pages/slug/about
```

Filter posts or pages by category/tag relation:

```http
GET /api/v1/posts?term_id=1
GET /api/v1/pages?term_id=2
```

Search posts or pages with a basic database LIKE query:

```http
GET /api/v1/posts?q=rust
GET /api/v1/pages?q=about
```

Slug values must use lowercase letters, numbers, and hyphens.

Post statuses:

- `draft`
- `pending_review`
- `published`
- `scheduled`
- `archived`

Authors can create drafts and pending-review content. Editors and admins can
publish or schedule content.

Schedule a post:

```http
PUT /api/v1/posts/1/status
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "status": "scheduled",
  "published_at": "2026-06-01T00:00:00Z"
}
```

Scheduled posts are hidden from normal lists until their `published_at` time.

Revisions:

```http
GET /api/v1/posts/1/revisions
Authorization: Bearer <access_token>
```

```http
PUT /api/v1/posts/1/revisions/3/restore
Authorization: Bearer <access_token>
```

The same routes are available under `/api/v1/pages/{id}/...`.

Term list responses include `post_count`:

```http
GET /api/v1/terms
```

## Comments

Public comment creation supports nested replies through `parent_id`. The parent
comment must belong to the same post.

Tiphia stores a salted SHA-256 hash of the client IP and a trimmed User-Agent for
moderation/audit use. These fields are not serialized in public comment
responses.

Comment creation is rate limited. Exceeding the limit returns:

```json
{
  "error": {
    "code": "rate_limited",
    "message": "rate limit exceeded"
  }
}
```

Rate limiting uses Redis when `TIPHIA_REDIS_URL` is set, for example:

```text
TIPHIA_REDIS_URL=redis://127.0.0.1:6379
```

If Redis is not configured, Tiphia falls back to in-memory limits for local
development.

Fetch approved comments as a nested tree:

```http
GET /api/v1/comments/post/{post_id}/tree
GET /api/v1/posts/{post_id}/comments/tree
GET /api/v1/pages/{page_id}/comments/tree
```

Moderation and flat comment listing require an editor or admin bearer token:

```http
GET /api/v1/comments?post_id=1&status=pending
Authorization: Bearer <access_token>

PUT /api/v1/comments/{id}/moderation
Authorization: Bearer <access_token>
```

## Settings

List theme schemas:

```http
GET /api/v1/themes
```

The response uses the same schema field shape as plugin configuration. Admin
frontends can render theme configuration forms dynamically from this endpoint.

Read site settings:

```http
GET /api/v1/settings
```

Update site settings, admin-only:

```http
PUT /api/v1/settings
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "title": "Tiphia",
  "description": "A Rust blog powered by Tiphia.",
  "avatar_url": "/assets/avatar.png",
  "base_url": "https://example.com",
  "timezone": "UTC",
  "default_page_size": 20,
  "comments_enabled": true,
  "comment_moderation": true,
  "registration_enabled": false,
  "permalink_format": "/posts/{slug}",
  "theme": {
    "active": "default",
    "configs": {
      "default": {
        "accent": "#2563eb"
      },
      "minimal": {
        "accent": "#111827"
      }
    },
    "config": {
      "accent": "#2563eb",
      "posts_per_page": 10,
      "custom_css": ".hero { text-align: center; }",
      "nav_pages": [
        { "label": "About", "slug": "about", "display": "article" },
        { "label": "Now", "slug": "now", "display": "plain" }
      ]
    }
  },
  "seo": {
    "meta_title_suffix": "Tiphia",
    "meta_description": "A Rust blog powered by Tiphia."
  }
}
```

`avatar_url` accepts `http://`, `https://`, or a site-root-relative path such as
`/assets/avatar.png`.

Theme activation is controlled by `theme.active`; user-defined theme options live
in `theme.configs` as per-theme free-form JSON. `theme.config` is the resolved
configuration for the active theme. The bundled admin UI edits the same settings
through the **主题** page and can delete an individual theme config.

The bundled blog frontend exposes primitive theme config values as CSS
variables. For example, `hero_layout` becomes `--theme-hero-layout`. A
`custom_css` string is injected as a style tag, so many visual changes can be
made without changing frontend source code.

`nav_pages` is an optional array for top-right custom page links. Each item uses:

- `label`: link text.
- `slug`: backend page slug requested through `GET /api/v1/pages/slug/{slug}`.
- `display`: `article` or `plain`; the frontend decides how to render the page.
