# Tiphia Unified Frontend

This app combines the admin console and the public blog frontend in one Vite
project.

- Admin routes live under `/admin`.
- Blog routes live under `/`.
- Admin source lives in `src/admin`.
- Blog source lives in `src/blog`.
- Shared routing and providers live in `src/shell`.
- Theme packages live in `src/themes/<theme-name>`.
- Frontend plugin packages live in `src/plugins/<plugin-name>`.

Theme package shape:

```text
src/themes/default/
  index.tsx
  theme.css
  README.md
```

The frontend skeleton provides hook slots through `src/framework/plugin-hooks`.
Themes decide where to render slots:

```tsx
import { FrontendHookSlot } from "../../framework/plugin-hooks";

<FrontendHookSlot hook="blog.footer.before" context={{ title }} />
```

Frontend plugin package shape:

```text
src/plugins/tiphia-links/
  index.tsx
  LinksConfigPanel.tsx
```

Plugins register admin panels, head effects, or rendered hook content:

```tsx
import { registerFrontendPlugin } from "../../framework/plugin-hooks";

registerFrontendPlugin({
  name: "tiphia-links",
  adminConfigPanel: LinksConfigPanel,
  hooks: [
    {
      hook: "blog.footer.before",
      render: () => <span>Plugin content</span>,
    },
  ],
});
```

Composition order:

```text
theme <- frontend skeleton <- frontend plugin logic <- backend plugin
```

The backend plugin may expose APIs, but a frontend plugin can also be purely
client-side if the author wants.

Run locally:

```bash
yarn install
yarn dev
```

Build:

```bash
yarn build
```
