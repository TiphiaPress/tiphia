# Release Checklist

Use this checklist before tagging a Tiphia release.

## Automated Checks

Run the checks from the repository root:

```bash
cargo check --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
yarn --cwd frontend build
```

Optional frontend dependency review:

```bash
yarn --cwd frontend audit --level moderate
```

## Docker

The Docker image is backend-only. It contains the API server and the
`tiphia-typecho-import` CLI, but not the React frontend. Deploy `frontend/dist`
with your preferred static hosting or reverse proxy.

Build the image:

```bash
docker build -t tiphia:release-check .
```

Run it with a mounted config:

```bash
cp tiphia.example.toml tiphia.toml
docker run --rm -p 3000:3000 \
  -e TIPHIA_JWT_SECRET=change-this-secret-before-production \
  -v "$PWD/tiphia.toml:/app/tiphia.toml:ro" \
  -v tiphia-data:/app/data \
  -v tiphia-logs:/app/logs \
  tiphia:release-check
```

## Security Review

- Set `app.environment = "production"` for production deployments.
- Change `auth.jwt_secret`; never use the development default.
- Set `cors.allowed_origins` to exact admin/blog origins.
- Keep `rate_limit.redis_url` configured for multi-instance deployments.
- Confirm `/openapi.json` is valid OpenAPI 3.0.
- Confirm plugin public routes return CORS headers.
- Confirm rendered HTML is sanitized by the backend renderer.
- Confirm plugin HTML output such as filing snippets is sanitized.
- Confirm CSP still allows required image sources but keeps `frame-ancestors 'none'`.

## Release Artifacts

At minimum, a release should include:

- The `tiphia` server binary.
- The `tiphia-typecho-import` migration CLI binary.
- `tiphia.example.toml` and `.env.example`.
- `README.md` and `docs/`.
- Plugin and theme README files that describe their configuration contracts.

Frontend release artifacts should be built and deployed separately from the
backend image:

```bash
yarn --cwd frontend build
```

## Smoke Test

After starting the release build:

1. Open `/health`.
2. Open `/openapi.json`.
3. Bootstrap a root user.
4. Log in to `/admin`.
5. Create one post and one page.
6. Create a category and tag, then bind them to the post.
7. Submit and moderate one comment.
8. Enable and configure the built-in plugins you intend to ship.
9. Open the public blog and confirm content, comments, plugin content, and theme config render correctly.
