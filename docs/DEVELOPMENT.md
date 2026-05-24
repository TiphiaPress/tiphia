# 后端开发约定

本文档记录后端核心开发中必须保持稳定的契约。它面向维护 Tiphia 后端、插件和迁移工具的开发者。

## 模块边界

后端核心位于 `crates/tiphia-core`，主要边界如下：

```text
crates/tiphia-core/src/routes/      HTTP 路由、请求解析、OpenAPI 注解
crates/tiphia-core/src/services/    业务服务、权限校验、数据读写
crates/tiphia-core/src/entities.rs  SeaORM 实体定义
crates/tiphia-core/src/migration.rs 数据库迁移
crates/tiphia-core/src/plugins.rs   后端插件 trait、注册、hook 调度
plugins/                            编译期后端插件
```

推荐规则：

- 路由层只做请求/响应转换，不写复杂业务逻辑。
- 服务层负责权限、校验、事务和实体操作。
- 共享行为优先放进服务层，不要在多个路由中复制。
- 插件需要持久化数据时，优先使用插件自己的配置、路由或迁移，而不是把结构塞进核心模型。

## Options 表契约

`options` 表用于保存系统级、插件级和主题级的轻量配置。它是框架契约的一部分，不是随意 key-value 存储。新增 key 前必须明确：

- key 命名格式。
- value JSON 结构。
- 谁负责读写。
- 是否需要迁移旧数据。
- 是否会被前端、主题、插件或导入工具依赖。

当前核心 key：

| Key | Value 结构 | 用途 |
| --- | --- | --- |
| `site:settings` | `SiteSettings` JSON | 站点标题、描述、头像、Gravatar 镜像、公开注册、SEO、主题配置等。 |
| `plugin:{plugin_name}:state` | `{ "enabled": boolean }` | 插件启用状态。后台插件页读写。 |
| `plugin:{plugin_name}:config` | 插件自定义 JSON | 插件配置。结构由插件 schema 和插件 README 说明。 |
| `post:view:{post_id}` | `{ "count": number }` | 文章阅读次数统计。公开文章详情访问时累加，热门文章和默认主题热门组件会读取它。 |

## 文章阅读次数

文章阅读次数当前不放在 `posts` 表字段中，而是保存在 `options` 表：

```text
post:view:{post_id}
```

value 示例：

```json
{
  "count": 1234
}
```

维护规则：

- 核心代码、主题依赖、统计服务和迁移工具必须使用同一个 key 格式：`post:view:{post_id}`。
- Typecho 迁移工具导入浏览量时，也必须写入该 key，不能写入其它平行 key。
- 不要新增 `views:{id}`、`post_views:{id}`、`post:{id}:views` 等替代 key，否则热门文章、默认主题和导入工具会出现不一致。
- 如果未来要把阅读次数迁移到独立表或 Redis，需要提供兼容读取和一次性迁移，不能直接删除旧 key。
- 阅读次数是统计数据，不应影响文章发布状态、权限或内容版本。

## 插件配置

插件启用状态和配置分开保存：

```text
plugin:{plugin_name}:state
plugin:{plugin_name}:config
```

插件作者需要在插件 README 中说明：

- 配置 JSON 示例。
- 每个字段的类型、默认值、是否必填。
- 插件公开 API 路径和响应结构。
- 插件后端 hook 与前端 hook 如何协作。

插件后端名称应保持稳定。前端插件可以通过 `backendNames` 兼容不同名称，但后端 option key 应以实际后端插件名为准。

## 主题配置

主题配置嵌套在 `site:settings.theme` 中，而不是单独拆成多个 option key。结构示例：

```json
{
  "active": "default",
  "configs": {
    "default": {
      "accent": "#2563eb"
    }
  },
  "config": {
    "accent": "#2563eb"
  }
}
```

字段说明：

- `active`：当前启用的主题配置名。为空字符串表示未启用任何主题配置。
- `configs`：所有已保存的主题配置。
- `config`：当前启用配置的解析结果，用于兼容旧读取方式。

新增主题字段时，应同步更新前端默认主题 README 和 `tiphia-docs`。

## 权限约定

后端权限是最终安全边界。前端隐藏页面只用于优化用户体验。

默认角色：

- `root`：最高管理员，可以管理其它管理员和系统级配置。
- `admin`：管理员，但不能管理 root 或同级管理员之间的高危操作。
- `editor`：内容编辑，管理文章、页面、评论、分类和标签。
- `author`：作者角色，默认不进入后台管理。

新增接口时必须明确：

- 是否公开。
- 是否需要登录。
- 需要哪个最低角色。
- 是否需要资源所有权检查。
- 是否允许插件 hook 修改行为。

## 迁移工具约定

迁移工具位于 `tools/`，它写入的核心数据必须遵守当前后端契约。尤其是：

- 文章、页面、分类、标签和评论要走当前实体结构。
- 分类和标签 slug 需要去重，避免唯一索引冲突。
- 文章浏览量必须写入 `options.post:view:{post_id}` 对应 key。
- 迁移工具不删除已有 Tiphia 数据，执行前应先备份源库和目标库。