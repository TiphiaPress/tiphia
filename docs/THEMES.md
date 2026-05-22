# Tiphia Themes

Tiphia core does not own frontend theme files. A theme is an independent
frontend app that reads the REST API, especially `/api/v1/settings`, and decides
how to render the active JSON config.

The admin console stores named theme configs under site settings:

```json
{
  "theme": {
    "active": "my-theme",
    "configs": {
      "my-theme": {
        "accent": "#2563eb",
        "posts_per_page": 10
      }
    },
    "config": {
      "accent": "#2563eb",
      "posts_per_page": 10
    }
  }
}
```

- `theme.active`: active config name.
- `theme.configs`: all saved config objects keyed by name.
- `theme.config`: resolved config for the active name.

No config is enabled by default. Administrators create and enable configs from
the admin theme page.

## Bundled Default Theme

The bundled default theme lives in `frontend/src/themes/default/` and
understands these keys:

```json
{
  "accent": "#2563eb",
  "font_family": "Inter, system-ui, sans-serif",
  "posts_per_page": 10,
  "show_popular_posts": true,
  "popular_posts_limit": 5,
  "show_recent_comments": true,
  "recent_comments_limit": 5,
  "favicon_url": "/themes/my-theme/favicon.ico",
  "custom_css": ".hero { text-align: center; }",
  "nav_pages": [
    { "label": "About", "slug": "about", "display": "article" },
    { "label": "Links", "slug": "links", "display": "article" }
  ]
}
```

Primitive values are also exposed as CSS variables using the key name. For
example `hero_layout` becomes `--theme-hero-layout`.

Use `favicon_url` or `favicon` to provide the browser tab icon for a theme. If a
theme does not provide one, the frontend falls back to the project favicon at
`/favicon.ico`.

## Plugin Data

Plugin-powered content is detected from plugin routes, not from theme switches.

- `tiphia-filing` exposes `/api/v1/filing`; the bundled blog renders it in the footer.
- `tiphia-links` exposes `/api/v1/links`; the bundled blog renders it on the custom page whose slug is `links`.

To show friend links in the bundled blog:

1. Enable and configure `tiphia-links`.
2. Create a backend page with slug `links`.
3. Add `{ "label": "友情链接", "slug": "links", "display": "article" }` to `nav_pages`.

The page content appears above grouped friend-link cards.
