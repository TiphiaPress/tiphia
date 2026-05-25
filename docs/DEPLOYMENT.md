# 部署方式

TiphiaPress 推荐后端和前端分开部署。后端只提供 API、认证、插件后端能力、迁移工具和日志；前端是独立静态资源，应单独构建并部署到静态托管或 Nginx。

## Release 配置文件

生产环境不要直接使用 `tiphia.example.toml`。建议复制一份到宿主机持久目录，例如：

```bash
sudo mkdir -p /opt/tiphia/{data,logs,config}
sudo cp tiphia.example.toml /opt/tiphia/config/tiphia.toml
```
容器内后端进程使用非 root 用户运行。镜像固定用户为 UID/GID `10001`，因此宿主机持久目录需要授予该用户写入权限：

```bash
sudo chown -R 10001:10001 /opt/tiphia/data /opt/tiphia/logs
sudo chmod 750 /opt/tiphia/data /opt/tiphia/logs
```

`config` 目录只需要宿主机可读，容器中以只读方式挂载即可。

然后将 `/opt/tiphia/config/tiphia.toml` 修改为 release 配置。一个 SQLite 单机部署示例：

```toml
[app]
environment = "production"

[http]
# Docker 容器内必须监听 0.0.0.0，否则宿主机端口映射无法访问。
bind = "0.0.0.0:3000"
request_timeout_secs = 30
max_body_bytes = 1048576

[cors]
# 只填写真实前端来源，不要在生产环境放开为 *。
allowed_origins = [
  "https://blog.example.com",
  "https://admin.example.com"
]

[database]
# Docker 推荐把 SQLite 文件放到 /app/data，并挂载为持久卷。
url = "sqlite:///app/data/tiphia.db?mode=rwc"
max_connections = 16
min_connections = 1
connect_timeout_secs = 10
acquire_timeout_secs = 10

[log]
level = "tiphia=info,tower_http=info"
# Docker 推荐挂载 /app/logs 到宿主机或 volume。
directory = "/app/logs"
file_prefix = "tiphia"
json = true

[auth]
# 必须改成高强度随机值。不要提交到 Git。
jwt_secret = "replace-with-a-long-random-secret"
token_ttl_seconds = 604800

[rate_limit]
# 单实例可以为空使用内存限流；多实例或生产高并发建议 Redis。
redis_url = ""
login_per_minute = 5
comments_per_minute = 10
```

生成 JWT secret 示例：

```bash
openssl rand -base64 48
```

Windows PowerShell 示例：

```powershell
[Convert]::ToBase64String((1..48 | ForEach-Object { Get-Random -Maximum 256 }))
```

配置加载规则：

- 默认读取当前工作目录下的 `tiphia.toml`。
- Docker 中建议设置 `TIPHIA_CONFIG=/app/tiphia.toml`。
- 环境变量优先级高于配置文件，可用于覆盖敏感项或容器部署项。

常用环境变量：

| 环境变量 | 对应配置 | 说明 |
| --- | --- | --- |
| `TIPHIA_CONFIG` | 配置文件路径 | 指向容器或宿主机内的配置文件。 |
| `TIPHIA_ENV` | `app.environment` | release 使用 `production`。 |
| `TIPHIA_BIND` | `http.bind` | Docker 内通常为 `0.0.0.0:3000`。 |
| `DATABASE_URL` | `database.url` | 覆盖数据库连接。 |
| `TIPHIA_LOG_DIR` | `log.directory` | 覆盖日志目录。 |
| `RUST_LOG` | `log.level` | 覆盖日志级别。 |
| `TIPHIA_JWT_SECRET` | `auth.jwt_secret` | 推荐通过环境变量注入生产 secret。 |
| `TIPHIA_CORS_ALLOWED_ORIGINS` | `cors.allowed_origins` | 逗号分隔的前端来源列表。 |
| `TIPHIA_REDIS_URL` | `rate_limit.redis_url` | Redis 限流地址。 |

## Docker Release 部署

构建镜像：

```bash
docker build -t tiphia:release .
```

运行容器：

```bash
docker run -d \
  --name tiphia \
  --restart unless-stopped \
  -p 3000:3000 \
  -e TIPHIA_CONFIG=/app/tiphia.toml \
  -e TIPHIA_ENV=production \
  -e TIPHIA_BIND=0.0.0.0:3000 \
  -e TIPHIA_JWT_SECRET="$(openssl rand -base64 48)" \
  -e TIPHIA_CORS_ALLOWED_ORIGINS="https://blog.example.com,https://admin.example.com" \
  -v /opt/tiphia/config/tiphia.toml:/app/tiphia.toml:ro \
  -v /opt/tiphia/data:/app/data \
  -v /opt/tiphia/logs:/app/logs \
  tiphia:release
```

说明：

- `--restart unless-stopped`：宿主机重启或进程异常退出后自动拉起。
- `tiphia.toml:/app/tiphia.toml:ro`：配置只读挂载。
- `/opt/tiphia/data:/app/data`：持久化 SQLite 数据库。
- `/opt/tiphia/logs:/app/logs`：持久化后端日志。

查看日志：

```bash
docker logs -f tiphia
```

重启服务：

```bash
docker restart tiphia
```

更新配置后也使用 `docker restart tiphia` 让服务重新读取配置。

## Docker Compose Release 示例

```yaml
services:
  tiphia:
    image: tiphia:release
    build: .
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      TIPHIA_CONFIG: /app/tiphia.toml
      TIPHIA_ENV: production
      TIPHIA_BIND: 0.0.0.0:3000
      DATABASE_URL: sqlite:///app/data/tiphia.db?mode=rwc
      TIPHIA_LOG_DIR: /app/logs
      TIPHIA_JWT_SECRET: change-this-secret-before-production
      TIPHIA_CORS_ALLOWED_ORIGINS: https://blog.example.com,https://admin.example.com
    volumes:
      - /opt/tiphia/config/tiphia.toml:/app/tiphia.toml:ro
      - /opt/tiphia/data:/app/data
      - /opt/tiphia/logs:/app/logs
```

启动：

```bash
docker compose up --build -d
```

停止：

```bash
docker compose down
```


## Redis 限流配置

Redis 用于保存登录和评论限流状态。单实例小站点可以留空 `redis_url` 使用内存限流；生产环境、Docker 多容器、横向扩容或需要重启后保留限流窗口时，建议配置 Redis。

### 配置文件方式

在 `tiphia.toml` 中配置：

```toml
[rate_limit]
redis_url = "redis://redis:6379/0"
login_per_minute = 5
comments_per_minute = 10
```

字段说明：

| 字段 | 说明 |
| --- | --- |
| `redis_url` | Redis 连接地址。为空字符串时使用内存限流。 |
| `login_per_minute` | 同一限流 key 每分钟允许的登录尝试次数。 |
| `comments_per_minute` | 同一限流 key 每分钟允许的评论提交次数。 |

常见 Redis URL：

```text
redis://127.0.0.1:6379/0
redis://redis:6379/0
redis://:password@redis:6379/0
rediss://:password@redis.example.com:6380/0
```

如果 Redis 设置了密码，推荐使用：

```toml
[rate_limit]
redis_url = "redis://:your-password@redis:6379/0"
```

### 环境变量方式

也可以通过环境变量覆盖配置文件：

```bash
-e TIPHIA_REDIS_URL="redis://redis:6379/0" \
-e TIPHIA_RATE_LIMIT_LOGIN_PER_MINUTE=5 \
-e TIPHIA_RATE_LIMIT_COMMENTS_PER_MINUTE=10
```

环境变量优先级高于 `tiphia.toml`。

### Docker Compose 示例

```yaml
services:
  tiphia:
    image: tiphia:release
    build: .
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      TIPHIA_CONFIG: /app/tiphia.toml
      TIPHIA_ENV: production
      TIPHIA_BIND: 0.0.0.0:3000
      DATABASE_URL: sqlite:///app/data/tiphia.db?mode=rwc
      TIPHIA_LOG_DIR: /app/logs
      TIPHIA_JWT_SECRET: change-this-secret-before-production
      TIPHIA_CORS_ALLOWED_ORIGINS: https://blog.example.com,https://admin.example.com
      TIPHIA_REDIS_URL: redis://redis:6379/0
    volumes:
      - /opt/tiphia/config/tiphia.toml:/app/tiphia.toml:ro
      - /opt/tiphia/data:/app/data
      - /opt/tiphia/logs:/app/logs
    depends_on:
      - redis

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data

volumes:
  redis-data:
```

如果需要 Redis 密码：

```yaml
services:
  redis:
    image: redis:7-alpine
    restart: unless-stopped
    command: redis-server --appendonly yes --requirepass your-password
    volumes:
      - redis-data:/data

  tiphia:
    environment:
      TIPHIA_REDIS_URL: redis://:your-password@redis:6379/0
```

### 配置建议

- 单机测试：`redis_url = ""` 即可。
- 单机生产：建议启用 Redis，避免服务重启后限流状态丢失。
- 多实例生产：必须启用 Redis，否则每个后端实例只会限制自己的内存窗口。
- Redis 不要直接暴露到公网；如果必须跨机器访问，请使用防火墙、内网、密码和 TLS。
- 登录限流建议保持较低，例如 `5`；评论限流可根据站点流量调整，例如 `10` 到 `30`。

### 验证 Redis 是否生效

启动后查看后端日志，确认没有 Redis 连接错误：

```bash
docker logs -f tiphia
```

也可以进入 Redis 容器检查限流 key 是否写入：

```bash
docker compose exec redis redis-cli keys 'tiphia:*'
```

生产环境不建议频繁使用 `keys` 扫描大库，可以改用 `scan`：

```bash
docker compose exec redis redis-cli scan 0 match 'tiphia:*' count 20
```

## 外部数据库与 Redis

SQLite 适合单机、小站点或早期部署。生产多实例建议使用 PostgreSQL/MySQL，并启用 Redis 限流：

```toml
[database]
url = "postgres://tiphia:password@postgres:5432/tiphia"
max_connections = 32
min_connections = 2
connect_timeout_secs = 10
acquire_timeout_secs = 10

[rate_limit]
redis_url = "redis://redis:6379/0"
login_per_minute = 5
comments_per_minute = 10
```

也可以通过环境变量覆盖：

```bash
-e DATABASE_URL="postgres://tiphia:password@postgres:5432/tiphia" \
-e TIPHIA_REDIS_URL="redis://redis:6379/0"
```

## 二进制 Release 部署

构建：

```bash
cargo build --release --locked
```

运行：

```bash
TIPHIA_CONFIG=/opt/tiphia/config/tiphia.toml ./target/release/tiphia
```

建议用 systemd 托管，并将 `/opt/tiphia/data`、`/opt/tiphia/logs`、`/opt/tiphia/config` 作为持久目录。


## 常见问题：日志目录 Permission denied

如果容器不断重启，并看到类似错误：

```text
initializing rolling file appender failed: failed to create log file: Permission denied
```

说明 `/app/logs` 对容器内的 `tiphia` 用户不可写。修复宿主机挂载目录权限：

```bash
sudo mkdir -p /opt/tiphia/data /opt/tiphia/logs
sudo chown -R 10001:10001 /opt/tiphia/data /opt/tiphia/logs
sudo chmod 750 /opt/tiphia/data /opt/tiphia/logs
docker restart tiphia
```

如果使用 Docker 命名 volume 而不是宿主机目录，通常不会遇到这个问题，因为镜像构建时已经为 `/app/data` 和 `/app/logs` 设置了容器内权限。

## 前端部署

前端仓库单独构建：

```bash
cd tiphia-frontend
yarn install
yarn build
```

构建结果在 `dist/`。构建前设置后端 API 地址：

```bash
VITE_TIPHIA_API_BASE=https://api.example.com
```

如果前端和后端跨域，后端 `cors.allowed_origins` 或 `TIPHIA_CORS_ALLOWED_ORIGINS` 必须包含前端来源。

## 初始化 Root 用户

第一次启动后，通过 API 创建最高管理员 root：

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

初始化完成后，该接口会拒绝再次创建。

## Release 检查清单

- `app.environment` 为 `production`。
- `http.bind` 在 Docker 中为 `0.0.0.0:3000`。
- `auth.jwt_secret` 已替换为高强度随机值。
- `cors.allowed_origins` 只包含真实前端域名。
- 数据库目录或外部数据库已持久化。
- 日志目录已挂载并能保留重启前日志。
- 多实例部署已配置 Redis 限流。
- `/health` 返回正常。
- `/openapi.json` 可访问或已由网关按需限制。