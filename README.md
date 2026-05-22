# Tiphia

Tiphia is a Rust blog framework inspired by Typecho. It uses Axum, SeaORM,
structured tracing, REST APIs, a unified React frontend, and compile-time Rust
plugins.

## Features

- Posts and pages with drafts, review, publishing, scheduling, archives, and revisions.
- Nested comments with moderation, rate limiting, IP hashing, and User-Agent capture.
- Categories and tags through a unified term system.
- User roles: root, admin, editor, author.
- REST API-first design for frontend separation.
- Utoipa OpenAPI document at `/openapi.json`.
- Compile-time plugin system with migrations, routes, config schema, admin menu, backend hooks, frontend hooks, and i18n resources.
- Built-in plugins for audit logs, friend links, filing information, and GeeTest captcha.
- Unified React/Vite frontend with admin console, public blog, themes, and frontend plugin packages.
- SQLite by default, with SeaORM connection-pool configuration.
- Redis-backed rate limiting with in-memory fallback for local development.

## Quick Start

```powershell
Copy-Item tiphia.example.toml tiphia.toml
Copy-Item .env.example .env
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

## Frontend

```powershell
cd frontend
Copy-Item .env.example .env
yarn install
yarn dev
```

Open:

- Public blog: `http://127.0.0.1:5173`
- Admin console: `http://127.0.0.1:5173/admin`

Set `VITE_TIPHIA_API_BASE` in `frontend/.env` if the backend is not running at
`http://127.0.0.1:3000`.

## Docker

Docker deploys the backend API only. Build and deploy the `frontend/` app
separately, then point `VITE_TIPHIA_API_BASE` to this backend and configure
`TIPHIA_CORS_ALLOWED_ORIGINS` with the frontend origin.

```bash
cp tiphia.example.toml tiphia.toml
docker build -t tiphia:local .
docker run --rm -p 3000:3000 \
  -e TIPHIA_JWT_SECRET=change-this-secret-before-production \
  -v "$PWD/tiphia.toml:/app/tiphia.toml:ro" \
  -v tiphia-data:/app/data \
  -v tiphia-logs:/app/logs \
  tiphia:local
```

Or use Compose:

```bash
cp tiphia.example.toml tiphia.toml
docker compose up --build
```

For production, change `auth.jwt_secret`, set `app.environment = "production"`,
configure `cors.allowed_origins`, and mount persistent `/app/data` and
`/app/logs` volumes.

## Verification

```bash
cargo check --locked
cargo test --workspace --locked
yarn --cwd frontend build
docker build -t tiphia:release-check .
```

## Documentation

- [REST API](docs/API.md)
- [Plugin development](docs/PLUGINS.md)
- [Theme and frontend integration](docs/THEMES.md)
- [Unified frontend](docs/FRONTEND.md)
- [Typecho migration](docs/TYPECHO_IMPORT.md)
- [Release checklist](docs/RELEASE.md)
- OpenAPI runtime document: `/openapi.json`
- Postman collection: `postman/Tiphia.postman_collection.json`

## Built-In Plugins

The main binary currently registers these compile-time plugins:

- `tiphia-audit`: logs lifecycle events and can reject comments by configured words.
- `tiphia-links`: stores friend links and exposes `GET /api/v1/links`.
- `tiphia-filing`: stores ICP/public security filing data and exposes `GET /api/v1/filing`.
- `tiphia-geetest`: verifies GeeTest v4 captcha proofs for login, registration, and comments.

Plugins are disabled by default after installation. Enable the plugin state from
the admin plugin page, then configure it. Each plugin directory includes its own
README with configuration examples.

## Repository Layout

```text
crates/tiphia-core/           Core framework, API, services, migrations
plugins/                      Compile-time backend plugins
frontend/                     Unified React frontend
frontend/src/themes/          Frontend theme packages
frontend/src/plugins/         Frontend plugin packages
tools/tiphia-typecho-import/  Typecho migration CLI
docs/                         Project documentation
postman/                      Postman collection and local environment
```
