# Tiphia Filing Plugin

`tiphia-filing` stores ICP and public security filing information and exposes it
to public frontends.

## Public API

```text
GET /api/v1/filing
```

The route also accepts `/api/v1/filing/`.

## Admin Config

Open the admin plugin page, find `tiphia-filing`, and edit its config.

Expected JSON:

```json
{
  "icp_number": "京ICP备00000000号-1",
  "icp_url": "https://beian.miit.gov.cn/",
  "police_html": "<a href=\"https://www.beian.gov.cn/portal/registerSystemInfo?recordcode=11000000000000\" target=\"_blank\">京公网安备 11000000000000号</a>"
}
```

Fields:

- `icp_number`: ICP filing number text.
- `icp_url`: ICP filing URL. Usually `https://beian.miit.gov.cn/`.
- `police_html`: public security filing HTML snippet.

`police_html` is sanitized before it is returned by the public API.

The bundled blog frontend automatically detects this plugin and renders filing
information in the footer when at least one filing field has content.
