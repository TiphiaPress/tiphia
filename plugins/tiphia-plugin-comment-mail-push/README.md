# Tiphia Comment Mail Push Plugin

`tiphia-comment-mail-push` 是 TiphiaPress 的评论邮件推送与找回密码插件。插件由后端 Rust crate 和前端 React 插件组成，后端负责监听评论 Hook、发送 SMTP 邮件和处理找回密码 token，前端负责后台配置面板、登录页找回密码入口和独立重置密码页面。

## 功能概览

- 新评论邮件通知：有新评论时通知指定收件邮箱。
- 评论回复通知：评论被回复时通知原评论者。
- 自动通知文章/页面作者：可在指定收件邮箱之外，同时通知内容作者，重复邮箱会自动去重。
- 后台评论回复：配合核心的评论回复 API，管理员在后台回复评论后也可触发回复邮件。
- 找回密码：登录页展示找回密码入口，通过邮件链接进入插件提供的重置密码页面。
- SMTP 配置：支持发信人、Reply-To、账号密码、是否认证、none/SSL/TLS 加密模式。
- HTML 邮件模板：评论通知、评论回复、找回密码三类模板均可自定义，并支持自定义 CSS。
- 安全默认值：插件默认禁用，SMTP 配置不完整时不会发信；找回密码 token 只保存 hash。

## 目录结构

独立插件仓库结构：

```text
tiphia-plugin-comment-mail-push/
  backend/       Rust 后端插件 crate
  frontend/      前端插件代码和后台配置面板
  .gitignore
  LICENSE
  README.md
```

主项目内置插件结构：

```text
tiphia/plugins/tiphia-plugin-comment-mail-push/
  Cargo.toml
  README.md
  src/
```

前端集成目录：

```text
tiphia-frontend/src/plugins/tiphia-comment-mail-push/
  api.ts
  CommentMailPushConfigPanel.tsx
  config.ts
  index.tsx
  PasswordRecoveryEntry.tsx
  PasswordResetPage.tsx
  styles.css
```

## 后端插件

后端 crate 位于 `backend/`，或主项目的 `tiphia/plugins/tiphia-plugin-comment-mail-push/`。

主要文件：

| 文件 | 说明 |
| --- | --- |
| `src/lib.rs` | 插件 manifest、Hook 注册、路由注册、评论通知主流程。 |
| `src/config.rs` | 配置结构、默认值、模板默认内容和配置就绪判断。 |
| `src/schema.rs` | 后台自动配置表单 schema 和公开配置响应。 |
| `src/mailer.rs` | SMTP 发信、Reply-To、HTML 模板渲染、自定义 CSS 注入。 |
| `src/password_reset.rs` | 找回密码 token 生成、hash、过期时间和 URL 拼接。 |
| `src/routes.rs` | 公开配置、发送找回密码邮件、重置密码 API。 |

### 后端路由

插件挂载在 `/api/v1` 下：

```text
GET  /api/v1/comment-mail-push/config
POST /api/v1/comment-mail-push/password/forgot
POST /api/v1/comment-mail-push/password/reset
```

`GET /config` 返回前端可见配置状态：

```json
{
  "enabled": true,
  "comment_push_enabled": true,
  "password_reset_enabled": true
}
```

`POST /password/forgot` 请求示例：

```json
{
  "account": "admin@example.com"
}
```

响应始终尽量保持模糊，避免账号枚举：

```json
{
  "accepted": true
}
```

`POST /password/reset` 请求示例：

```json
{
  "token": "reset-token-from-email",
  "password": "new-long-password"
}
```

响应：

```json
{
  "reset": true
}
```

## Hook 行为

插件监听：

```rust
Hook::AfterCommentCreate
```

评论创建成功后，插件会按配置执行：

1. 若 `comment_push_enabled = true` 且 SMTP 配置完整，发送“评论通知”邮件。
2. 若 `notify_post_author_enabled = true`，除指定收件邮箱外，也通知文章/页面作者。
3. 若新评论有 `parent_id`，且 `comment_reply_enabled = true`，发送“评论回复通知”给被回复评论者。
4. 若收件邮箱重复，会自动去重。
5. 邮件发送失败只记录日志，不阻断评论创建。

## 配置字段

| 字段 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `comment_push_enabled` | boolean | `false` | 启用新评论邮件通知。 |
| `comment_reply_enabled` | boolean | `false` | 启用评论回复通知。 |
| `notify_post_author_enabled` | boolean | `false` | 自动通知文章/页面作者。 |
| `password_reset_enabled` | boolean | `false` | 启用找回密码功能。 |
| `reset_token_ttl_minutes` | number | `30` | 找回密码链接有效期，单位分钟，范围建议 10 到 60。 |
| `comment_notify_email` | string | `""` | 评论通知指定收件邮箱；为空时默认发送到 `from_email`。 |
| `from_name` | string | `"TiphiaPress"` | 发信人名称。 |
| `from_email` | string | `""` | 发件邮箱地址。 |
| `reply_to_email` | string | `""` | 邮件 Reply-To 地址；用户点击“回复”时会回到该邮箱。 |
| `password_reset_base_url` | string | `""` | 找回密码页面地址，通常为 `https://example.com/password-reset`，插件会追加 `token` 参数。 |
| `comment_email_template` | string | 内置模板 | 新评论通知 HTML 模板。 |
| `comment_reply_email_template` | string | 内置模板 | 评论回复通知 HTML 模板。 |
| `password_reset_email_template` | string | 内置模板 | 找回密码 HTML 模板。 |
| `email_custom_css` | string | `""` | 注入到邮件中的自定义 CSS。 |
| `smtp_host` | string | `""` | SMTP 服务器地址。 |
| `smtp_port` | number | `587` | SMTP 端口。 |
| `smtp_username` | string | `""` | SMTP 登录用户。 |
| `smtp_password` | string | `""` | SMTP 登录密码，建议使用应用专用密码。 |
| `smtp_auth_required` | boolean | `true` | 是否需要 SMTP 服务器认证。 |
| `smtp_encryption` | string | `"tls"` | 可选：`none`、`ssl`、`tls`。 |

## 配置 JSON 示例

```json
{
  "comment_push_enabled": true,
  "comment_reply_enabled": true,
  "notify_post_author_enabled": true,
  "password_reset_enabled": true,
  "reset_token_ttl_minutes": 30,
  "comment_notify_email": "admin@example.com",
  "from_name": "TiphiaPress",
  "from_email": "noreply@example.com",
  "reply_to_email": "support@example.com",
  "password_reset_base_url": "https://blog.example.com/password-reset",
  "comment_email_template": "<h2>收到新评论</h2><p>{{sender_name}} 评论了 <a href=\"{{post_url}}\">{{post_title}}</a></p><blockquote>{{comment_content}}</blockquote>",
  "comment_reply_email_template": "<h2>你的评论收到回复</h2><p>{{sender_name}} 回复了你：</p><blockquote>{{replied_content}}</blockquote><p>新回复：</p><blockquote>{{comment_content}}</blockquote>",
  "password_reset_email_template": "<h2>找回密码</h2><p>你好，{{display_name}}</p><p><a href=\"{{reset_url}}\">点击重置密码</a></p>",
  "email_custom_css": "body{font-family:system-ui,sans-serif}.button{background:#2563eb;color:white;padding:10px 14px;border-radius:6px}",
  "smtp_host": "smtp.example.com",
  "smtp_port": 587,
  "smtp_username": "noreply@example.com",
  "smtp_password": "replace-with-smtp-password",
  "smtp_auth_required": true,
  "smtp_encryption": "tls"
}
```

## SMTP 加密模式

| 模式 | 说明 | 常见端口 |
| --- | --- | --- |
| `none` | 无安全加密，仅建议内网或测试使用。 | 25 |
| `ssl` | 隐式 SSL，连接建立后立即加密。 | 465 |
| `tls` | STARTTLS，先建立连接再升级为 TLS。 | 587 |

常见邮箱服务建议：

- 465 端口通常选择 `ssl`。
- 587 端口通常选择 `tls`。
- 大多数公网邮箱都需要 `smtp_auth_required = true`。
- `smtp_password` 通常不是邮箱登录密码，而是服务商生成的 SMTP 授权码或应用专用密码。

## 邮件模板变量

### 评论通知模板

评论通知模板用于通知站点管理员、指定收件邮箱和文章/页面作者。

可用变量：

| 变量 | 含义 |
| --- | --- |
| `{{post_title}}` | 文章或页面标题。 |
| `{{post_url}}` | 文章或页面链接，按前端真实路由生成。文章为 `/posts/{slug}`，页面为 `/pages/{slug}`。 |
| `{{sender_name}}` | 本次评论的发送者名称。未登录时为评论者填写的昵称；登录时为登录用户显示名。 |
| `{{sender_email}}` | 本次评论的发送者邮箱。 |
| `{{commenter_name}}` | 兼容字段，等同于评论者名称。 |
| `{{commenter_email}}` | 兼容字段，等同于评论者邮箱。 |
| `{{post_author_name}}` | 文章/页面作者显示名。 |
| `{{post_author_email}}` | 文章/页面作者邮箱。 |
| `{{comment_status}}` | 评论状态。 |
| `{{comment_content}}` | 评论内容，已做 HTML 转义并转换换行。 |
| `{{author_name}}` | 旧模板兼容字段，等同于评论者名称。 |
| `{{author_email}}` | 旧模板兼容字段，等同于评论者邮箱。 |

默认模板：

```html
<h2>收到新评论</h2>
<p><strong>文章：</strong><a href="{{post_url}}">{{post_title}}</a></p>
<p><strong>评论者：</strong>{{sender_name}} &lt;{{sender_email}}&gt;</p>
<p><strong>文章作者：</strong>{{post_author_name}}</p>
<p><strong>状态：</strong>{{comment_status}}</p>
<blockquote>{{comment_content}}</blockquote>
```

### 评论回复模板

评论回复模板用于通知被回复的评论者。

可用变量：

| 变量 | 含义 |
| --- | --- |
| `{{post_title}}` | 文章或页面标题。 |
| `{{post_url}}` | 文章或页面链接。 |
| `{{sender_name}}` | 回复者名称。管理员后台回复时为管理员显示名；访客回复时为评论者填写的昵称。 |
| `{{sender_email}}` | 回复者邮箱。 |
| `{{recipient_name}}` | 被回复评论者名称。 |
| `{{recipient_email}}` | 被回复评论者邮箱。 |
| `{{replied_content}}` | 被回复的原评论内容。 |
| `{{comment_content}}` | 新回复内容。 |
| `{{author_name}}` | 旧模板兼容字段，等同于回复者名称。 |
| `{{author_email}}` | 旧模板兼容字段，等同于回复者邮箱。 |

默认模板：

```html
<h2>你的评论收到回复</h2>
<p>{{recipient_name}}，你好：</p>
<p>{{sender_name}} 回复了你在 <a href="{{post_url}}">{{post_title}}</a> 下的评论。</p>
<p><strong>你原来的评论：</strong></p>
<blockquote>{{replied_content}}</blockquote>
<p><strong>新的回复：</strong></p>
<blockquote>{{comment_content}}</blockquote>
```

### 找回密码模板

可用变量：

| 变量 | 含义 |
| --- | --- |
| `{{display_name}}` | 用户显示名。 |
| `{{reset_url}}` | 带 token 的重置密码链接。 |
| `{{ttl_minutes}}` | 链接有效期，单位分钟。 |

默认模板：

```html
<h2>找回密码</h2>
<p>你好，{{display_name}}：</p>
<p>请点击下面的链接重置密码。该链接将在 {{ttl_minutes}} 分钟后过期。</p>
<p><a href="{{reset_url}}">重置密码</a></p>
<p>如果不是你本人操作，可以忽略这封邮件。</p>
```

## 自定义 CSS

`email_custom_css` 会自动注入邮件 HTML。模板中可以显式写入：

```html
<style>{{custom_css}}</style>
```

如果模板没有写 `{{custom_css}}`，插件会尽量自动插入到 HTML 前部。建议把邮件 CSS 写成内联友好的简单样式，避免依赖复杂选择器、外部字体或脚本。

示例：

```css
body {
  margin: 0;
  background: #f6f8fb;
  color: #111827;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

.mail-card {
  max-width: 640px;
  margin: 24px auto;
  padding: 24px;
  border: 1px solid #dbe3ef;
  border-radius: 12px;
  background: #ffffff;
}

.button {
  display: inline-block;
  padding: 10px 14px;
  border-radius: 8px;
  background: #2563eb;
  color: #ffffff;
  text-decoration: none;
}
```

## 找回密码流程

1. 后台启用插件并开启 `password_reset_enabled`。
2. 填写完整 SMTP 配置。
3. 设置 `password_reset_base_url`，例如：

```text
https://blog.example.com/password-reset
```

4. 前端插件在登录页通过 `admin.auth.form.after` Hook 展示“忘记密码？”入口。
5. 用户填写账号或邮箱后，前端请求：

```text
POST /api/v1/comment-mail-push/password/forgot
```

6. 后端查找用户，生成 token，只保存 token hash。
7. 邮件中的链接形如：

```text
https://blog.example.com/password-reset?token=xxxx
```

8. 用户提交新密码后，前端请求：

```text
POST /api/v1/comment-mail-push/password/reset
```

9. 后端验证 token、检查过期时间、更新密码并清除 token 记录。

## 前端插件注册

前端插件入口：

```tsx
registerFrontendPlugin({
  name: "tiphia-comment-mail-push",
  backendNames: ["tiphia-comment-mail-push", "tiphia-plugin-comment-mail-push"],
  adminConfigPanel: CommentMailPushConfigPanel,
  routes: [
    {
      path: "/password-reset",
      element: <PasswordResetPage />,
    },
  ],
  hooks: [
    {
      hook: "admin.auth.form.after",
      order: 80,
      render: (context) => <PasswordRecoveryEntry mode={String(context.mode || "login")} />,
    },
  ],
});
```

这意味着：

- 后台插件页会显示该插件的配置面板。
- 登录页会在表单后展示找回密码入口。
- `/password-reset` 页面由插件提供，不复用登录页。

## 集成方式

### 已在主项目内时

如果插件已经位于 `tiphia/plugins/tiphia-plugin-comment-mail-push`，通常只需要：

1. 确认后端 `Cargo.toml` workspace 包含该插件。
2. 确认主程序注册该插件。
3. 确认前端 `src/plugins/index.ts` 导入 `./tiphia-comment-mail-push`。
4. 重新构建后端和前端。
5. 在后台插件页启用插件并保存配置。

### 作为独立插件仓库时

1. 将 `backend/` 作为 Rust crate 加入后端 workspace，或复制到：

```text
tiphia/plugins/tiphia-plugin-comment-mail-push
```

2. 在后端插件注册处调用：

```rust
tiphia_plugin_comment_mail_push::register(builder)?;
```

3. 将 `frontend/` 复制到：

```text
tiphia-frontend/src/plugins/tiphia-comment-mail-push
```

4. 在前端插件入口导入：

```ts
import "./tiphia-comment-mail-push";
```

5. 构建并部署。

## 日志与排错

插件会在发信失败时记录日志，日志里至少包含目标邮箱和错误信息。常见问题：

| 现象 | 可能原因 | 处理方式 |
| --- | --- | --- |
| 没有收到评论通知 | `comment_push_enabled` 未开启，或 SMTP 配置不完整。 | 检查插件配置和后端日志。 |
| 评论回复没有邮件 | `comment_reply_enabled` 未开启，或原评论没有邮箱，或回复者邮箱与原评论邮箱相同。 | 检查原评论邮箱和配置。 |
| 找回密码入口不显示 | 插件未启用，前端插件未导入，或公开配置接口不可访问。 | 检查 `/api/v1/comment-mail-push/config`。 |
| 找回密码邮件不发送 | `password_reset_enabled` 未开启，或 `password_reset_base_url` 为空。 | 补齐配置。 |
| SMTP 认证失败 | 用户名/密码错误，或需要应用专用密码。 | 到邮箱服务商后台生成授权码。 |
| 连接超时 | SMTP 地址、端口、加密模式不匹配，或服务器防火墙阻断。 | 465 用 `ssl`，587 用 `tls`，确认容器可访问外网。 |
| 邮件进垃圾箱 | SPF/DKIM/DMARC 未配置，或发件域名不可信。 | 配置域名邮件记录。 |

## 安全注意事项

- 插件默认禁用，必须在后台显式启用。
- 找回密码 token 只保存 hash，不保存明文。
- 找回密码接口不暴露账号是否存在，降低账号枚举风险。
- `smtp_password` 是敏感配置，生产环境应限制后台和数据库访问权限。
- `password_reset_base_url` 必须是可信前端地址，避免 token 被发送到错误域名。
- `reply_to_email` 只是邮件头里的 Reply-To，不是找回密码地址。
- 生产环境建议使用 `ssl` 或 `tls`，不要使用 `none`。
- 自定义 HTML 模板不要插入外部脚本，邮件客户端通常也会拦截脚本。

## 开发与验证

后端插件检查：

```bash
cd backend
cargo fmt
cargo check
```

主项目检查：

```bash
cd tiphia
cargo fmt
cargo check
```

前端构建：

```bash
cd tiphia-frontend
yarn build
```