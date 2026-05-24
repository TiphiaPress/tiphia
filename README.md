# Tiphia

Tiphia is the Rust backend of the TiphiaPress blog framework. It provides the REST API, authentication, content services, comments, taxonomy, settings, migrations, built-in backend plugins, and Docker deployment for a Typecho-inspired but API-first publishing system.

## Documentation

The full documentation is published at:

https://tiphiapress.github.io/

Useful sections:

- API reference: https://tiphiapress.github.io/#/api
- Deployment guide: https://tiphiapress.github.io/#/deployment
- Backend hooks: https://tiphiapress.github.io/#/backend-hooks
- Plugin development: https://tiphiapress.github.io/#/plugins
- Typecho migration: https://tiphiapress.github.io/#/migration
- Backend development notes: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)

## Repository Role

This repository contains only the backend side of TiphiaPress:

```text
crates/tiphia-core/           Core framework, routes, services, entities, migrations
plugins/                      Compile-time backend plugins
tools/tiphia-typecho-import/  Typecho migration CLI
docs/                         Source documentation snapshots
postman/                      Postman collection and local environment
```

The frontend and themes are maintained separately:

- Frontend shell: `TiphiaPress/tiphia-frontend`
- Default theme: `TiphiaPress/tiphia-default-themes`
- Documentation site: `TiphiaPress/tiphia-docs`

## Features

- Posts and pages with drafts, review, publishing, scheduling, archives, and revisions.
- Nested comments with moderation, rate limiting, IP hashing, and User-Agent capture.
- Categories and tags through a unified term system.
- User roles: root, admin, editor, author.
- REST API-first design for frontend separation.
- Utoipa OpenAPI document at `/openapi.json`.
- Compile-time plugin system with migrations, routes, config schema, backend hooks, admin menu metadata, and plugin-owned APIs.
- Built-in plugins for audit logs, friend links, filing information, and GeeTest captcha.
- SQLite by default, with SeaORM connection-pool configuration.
- Redis-backed rate limiting with in-memory fallback for local development.

## Quick Start

```bash
cp tiphia.example.toml tiphia.toml
cp .env.example .env
cargo run
```

Health check:

```http
GET /health
```

OpenAPI:

```http
GET /openapi.json
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

## Docker

Docker deploys the backend API only. Build and deploy `tiphia-frontend` separately, then set `VITE_TIPHIA_API_BASE` on the frontend and `TIPHIA_CORS_ALLOWED_ORIGINS` on the backend.

```bash
cp tiphia.example.toml tiphia.toml
docker build -t tiphia:local .
docker run --rm -p 3000:3000 \
  -e TIPHIA_JWT_SECRET=change-this-secret-before-production \
  -e TIPHIA_CORS_ALLOWED_ORIGINS=https://your-frontend.example.com \
  -v "$PWD/tiphia.toml:/app/tiphia.toml:ro" \
  -v tiphia-data:/app/data \
  -v tiphia-logs:/app/logs \
  tiphia:local
```

## Development Checks

```bash
cargo check --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```
