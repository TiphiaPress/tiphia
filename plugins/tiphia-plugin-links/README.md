# Tiphia Links Plugin

`tiphia-links` stores friend links and exposes them to public frontends.

## Public API

```text
GET /api/v1/links
```

The route also accepts `/api/v1/links/`.

## Admin Config

Open the admin plugin page, find `tiphia-links`, and edit its config.

Expected JSON:

```json
{
  "links": [
    {
      "name": "Rust",
      "description": "Rust programming language",
      "url": "https://www.rust-lang.org/",
      "avatar_url": "https://www.rust-lang.org/static/images/rust-logo-blk.svg",
      "category": "Tech"
    },
    {
      "name": "Friend Blog",
      "description": "A personal blog",
      "url": "https://example.com/",
      "avatar_url": "https://example.com/avatar.png",
      "category": "Friends"
    }
  ]
}
```

Fields:

- `name`: required display name.
- `description`: optional short description.
- `url`: required HTTP or HTTPS homepage URL.
- `avatar_url`: optional avatar/logo URL.
- `category`: optional user-defined group name.

The bundled blog frontend renders this plugin on the custom page whose slug is
`links`. Create a page with slug `links`; its content appears above the grouped
link cards.
