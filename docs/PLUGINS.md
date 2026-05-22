# Tiphia Plugin Guide

Tiphia plugins are normal Rust crates compiled together with the main server.
This gives each plugin its own package structure, dependencies, modules, tests,
and routes without runtime dynamic loading risk.

## Layout

```text
plugins/
  tiphia-plugin-audit/
    Cargo.toml
    src/lib.rs
  tiphia-plugin-links/
    Cargo.toml
    src/lib.rs
    README.md
```

Each plugin depends on `tiphia-core`:

```toml
[dependencies]
tiphia-core = { path = "../../crates/tiphia-core" }
```

The main binary depends on the plugin crate and registers it:

```rust
fn register_compiled_plugins(
    builder: &mut tiphia_core::plugins::PluginRegistryBuilder,
) -> tiphia_core::AppResult<()> {
    tiphia_plugin_audit::register(builder)?;
    tiphia_plugin_links::register(builder)?;
    tiphia_plugin_filing::register(builder)
}
```

## Plugin Contract

Plugins implement `tiphia_core::plugins::Plugin`.

Important extension points:

- `manifest`: plugin metadata exposed by `/api/v1/plugins`.
- `install`: initialize plugin-owned tables, options, or seed data.
- `migrations`: declares plugin-owned database migrations.
- `hooks`: declares hook subscriptions and priorities.
- `admin_menu`: declares admin UI entries.
- `config_schema`: declares plugin settings fields.
- `activate`: runs once during application boot.
- `handle`: receives lifecycle hook events.
- `routes`: lets the plugin mount its own REST endpoints.

Lower hook priority values run earlier.

## Mutable Hook Context

Before hooks can modify the payload that the core service will continue with:

```rust
async fn handle(&self, hook: Hook, context: &mut HookContext) -> AppResult<()> {
    if matches!(hook, Hook::BeforeCommentCreate) {
        if let Some(mut input) = context.subject_as::<CreateCommentInput>()? {
            input.content = input.content.trim().to_owned();
            context.replace_subject(input)?;
        }
    }

    Ok(())
}
```

Hooks can also stop the workflow:

```rust
context.stop("comment rejected by plugin");
```

When a hook stops execution, later listeners are skipped and the core operation
returns an error.

Hook handlers can access the database through the context:

```rust
let db = context.database()?;
```

This is useful for audit logs, plugin-owned tables, counters, caches, and other
stateful extensions.

## Current Hooks

- `AppBooting`
- `AppBooted`
- `RequestReceived`
- `BeforePostList`
- `AfterPostList`
- `BeforePostCreate`
- `AfterPostCreate`
- `BeforePostUpdate`
- `AfterPostUpdate`
- `BeforePostDelete`
- `AfterPostDelete`
- `BeforePageList`
- `AfterPageList`
- `BeforePageCreate`
- `AfterPageCreate`
- `BeforeCommentCreate`
- `AfterCommentCreate`
- `BeforeCommentModerate`
- `AfterCommentModerate`
- `BeforeTermCreate`
- `AfterTermCreate`
- `BeforeTermUpdate`
- `AfterTermUpdate`
- `BeforeTermDelete`
- `AfterTermDelete`
- `BeforePostTermsSync`
- `AfterPostTermsSync`
- `BeforeRender`
- `AfterRender`
- `AdminMenu`

`BeforeRender` receives `RenderInput`; `AfterRender` receives
`RenderedContent`. Plugins can modify either payload through `replace_subject`.

## Config Helpers

Most plugins store JSON config in the core `options` table. Use the shared
helpers from `tiphia_core::plugins` instead of hand-writing option lookup and
merge code in every plugin:

```rust
use tiphia_core::plugins::{ensure_plugin_config, load_plugin_config};

async fn install(&self, db: &DatabaseConnection) -> AppResult<()> {
    ensure_plugin_config(db, self.manifest().name, serde_json::json!(MyConfig::default())).await
}

async fn load_config(db: &DatabaseConnection, plugin_name: &str) -> AppResult<MyConfig> {
    load_plugin_config(db, plugin_name, MyConfig::default()).await
}
```

`load_plugin_config` merges stored values onto the default config and also
accepts the admin API envelope shape `{ "config": { ... } }`.

Use `load_plugin_config_with` when a plugin needs to normalize legacy or
user-friendly JSON shapes before deserializing.

## Examples

See `plugins/tiphia-plugin-audit`.

The audit plugin demonstrates runtime plugin config:

- `log_post_events`
- `log_comment_events`
- `blocked_comment_words`

Other bundled examples:

- `plugins/tiphia-plugin-links`: public route plus JSON array config.
- `plugins/tiphia-plugin-filing`: public route plus sanitized HTML output.

## Plugin Management API

- `GET /api/v1/plugins`
- `GET /api/v1/plugins/admin-menu`
- `GET /api/v1/plugins/{name}/config`
- `PUT /api/v1/plugins/{name}/config`

Plugin config is stored in the core `options` table under:

```text
plugin:<plugin-name>:config
```

## Plugin Migrations

Plugins can return versioned migrations from `migrations()`. Tiphia stores applied
migration ids in `schema_migrations`, so each migration runs once.

```rust
fn migrations(&self) -> Vec<SharedMigration> {
    vec![Box::new(CreatePluginTable)]
}
```

Migration ids should be globally unique:

```text
plugin:<plugin-name>:0001:create-table
```

Each migration runs inside a transaction. Plugins can optionally implement
`down` to support rollback:

```rust
async fn down(&self, db: &DatabaseTransaction) -> AppResult<()> {
    // drop plugin-owned tables or undo reversible changes
    Ok(())
}
```

If `down` is omitted, rollback reports that the migration is not reversible.

## Plugin Config Validation

`config_schema` is enforced when `/api/v1/plugins/{name}/config` is updated.
Required fields must be present, and field values must match their declared
types:

- `Text` / `Textarea`: string
- `Number`: number
- `Boolean`: boolean
- `Json`: any JSON value

Unknown fields are currently allowed so plugins can evolve configs without
breaking older clients.

## Hook Observability

Hook dispatch emits structured tracing fields including plugin name, hook,
priority, elapsed milliseconds, and whether execution was stopped. Failed hooks
are logged with the error.
