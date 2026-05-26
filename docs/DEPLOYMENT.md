# 部署方式

TiphiaPress 推荐后端和前端分开部署。后端只提供 API、认证、插件后端能力、迁移工具和日志；前端是独立静态资源，应单独构建并部署到静态托管或 Nginx。

## 部署场景速查

| 场景 | 后端 | 前端 | API Base | Nginx/CORS 要点 |
| --- | --- | --- | --- | --- |
| 单机 Docker + Nginx | Docker 暴露 `127.0.0.1:7999` | Nginx 静态目录 | `VITE_TIPHIA_API_BASE=` | Nginx `location /api/` 反代到后端；后端 CORS 写前端域名。 |
| 前后端同域同 Nginx | 后端仅监听内网端口 | 同一个域名静态资源 | 空 | 最推荐；浏览器请求 `/api/v1/...`，不产生跨域。 |
| 前后端不同域 | 后端独立域名 `api.example.com` | 前端独立域名 `blog.example.com` | `https://api.example.com` | 后端 `cors.allowed_origins` 必须包含 `https://blog.example.com`。 |
| CDN/对象存储静态部署 | 后端在 VPS 或容器 | CDN/对象存储 | 独立 API 域名或运行时覆盖 | 不能依赖同源 `/api/`，除非 CDN 支持路径回源到后端。 |
| 本地开发 | `cargo run` | `yarn dev` | `http://127.0.0.1:3000` 或 Vite proxy | 开发环境可用本地地址；不要把该值打进生产包。 |
| 内网/反代多实例 | 多个后端实例 + Redis | 静态资源 | 同源或独立 API | 多实例必须配置 Redis 限流；反代要保留 `X-Forwarded-*`。 |

选择原则：能同源就同源，能让前端请求 `/api/` 就不要把 `127.0.0.1`、内网 IP 或临时端口写进生产前端包。
## Release 配置文件

生产环境不要直接使用 `tiphia.example.toml`。建议复制一份到宿主机持久目录，例如：

```bash
sudo mkdir -p /opt/tiphia/{data,logs,config}
sudo cp tiphia.example.toml /opt/tiphia/config/tiphia.toml
```
容器启动时会自动修正 `/app/data` 和 `/app/logs` 的权限，然后降权为 `tiphia` 用户运行。镜像内 `tiphia` 用户固定为 UID/GID `10001`。

如果你使用旧镜像，或宿主机安全策略禁止容器修改挂载目录权限，可以手动执行：

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

### 只使用配置文件

如果你希望 release 环境完全由 `tiphia.toml` 管理，可以把 `jwt_secret`、数据库、日志、CORS、Redis 等全部写入配置文件，然后启动时不再传 `DATABASE_URL`、`TIPHIA_JWT_SECRET`、`TIPHIA_CORS_ALLOWED_ORIGINS`、`TIPHIA_REDIS_URL` 等覆盖变量。

Docker 推荐仍然把配置文件挂载到镜像默认工作目录的 `/app/tiphia.toml`：

```bash
docker run -d \
  --name tiphia \
  --restart unless-stopped \
  -p 7999:3000 \
  -v /etc/tiphia/config/tiphia.toml:/app/tiphia.toml:ro \
  -v /etc/tiphia/data:/app/data \
  -v /etc/tiphia/logs:/app/logs \
  tiphia:latest
```

这种方式下需要确保配置文件里至少包含：

```toml
[app]
environment = "production"

[http]
bind = "0.0.0.0:3000"

[cors]
allowed_origins = ["https://posts.example.com"]

[database]
url = "sqlite:///app/data/tiphia.db?mode=rwc"

[log]
directory = "/app/logs"
json = true

[auth]
jwt_secret = "replace-with-a-long-random-secret"

[rate_limit]
redis_url = ""
```

注意：环境变量优先级高于配置文件。如果你已经在 `docker run`、`docker compose.yml`、systemd 或宿主机环境里设置了同名覆盖变量，最终生效的会是环境变量，而不是 TOML。
### Docker Compose 只使用配置文件

如果使用 Compose，也可以只挂载 TOML，不在 `environment` 中写业务配置：

```yaml
services:
  tiphia:
    image: tiphia:latest
    restart: unless-stopped
    ports:
      - "7999:3000"
    volumes:
      - /etc/tiphia/config/tiphia.toml:/app/tiphia.toml:ro
      - /etc/tiphia/data:/app/data
      - /etc/tiphia/logs:/app/logs
```

这种方式的优点是所有 release 配置都集中在 `/etc/tiphia/config/tiphia.toml`，便于备份和审计。缺点是敏感信息如 `jwt_secret`、数据库密码也会写在 TOML 中，所以该文件权限应限制为管理员可读：

```bash
sudo chown root:root /etc/tiphia/config/tiphia.toml
sudo chmod 600 /etc/tiphia/config/tiphia.toml
```

如果你希望敏感项不落盘，可以只把 `TIPHIA_JWT_SECRET`、`DATABASE_URL`、`TIPHIA_REDIS_URL` 放到环境变量，其余配置仍放在 TOML。这是“配置文件为主，敏感项环境变量覆盖”的折中方式。

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

说明 `/app/logs` 对容器内的 `tiphia` 用户不可写。新版镜像会在启动时自动修正 `/app/data` 和 `/app/logs` 权限；如果你仍然看到这个错误，通常是还在运行旧镜像，或者容器不是用默认 entrypoint 启动。

推荐重新构建并重建容器：

```bash
docker build -t tiphia:latest .
docker rm -f tiphia
docker run -d \
  --name tiphia \
  --restart unless-stopped \
  -p 7999:3000 \
  -e TIPHIA_CONFIG=/app/tiphia.toml \
  -e TIPHIA_ENV=production \
  -e TIPHIA_BIND=0.0.0.0:3000 \
  -e TIPHIA_CORS_ALLOWED_ORIGINS="https://posts.cairbin.top" \
  -v /etc/tiphia/config/tiphia.toml:/app/tiphia.toml:ro \
  -v /etc/tiphia/data:/app/data \
  -v /etc/tiphia/logs:/app/logs \
  tiphia:latest
```

如果暂时不重建镜像，可以手动修复宿主机目录权限：

```bash
sudo mkdir -p /etc/tiphia/data /etc/tiphia/logs
sudo chown -R 10001:10001 /etc/tiphia/data /etc/tiphia/logs
sudo chmod 750 /etc/tiphia/data /etc/tiphia/logs
docker restart tiphia
```

如果使用 Docker 命名 volume 而不是宿主机目录，通常不会遇到这个问题。

## Nginx 反向代理

推荐生产部署采用“前端静态资源 + 同源 `/api/` 反代后端”的方式：

- 用户访问 `https://posts.example.com/` 时由 Nginx 返回前端静态文件。
- 用户访问 `https://posts.example.com/api/...` 时由 Nginx 转发到后端容器或后端进程。
- 前端构建时 `VITE_TIPHIA_API_BASE` 留空，浏览器会自动请求同源 `/api/v1/...`。

一个完整 Nginx server 示例：

```nginx
server {
    listen 80;
    server_name posts.example.com;

    root /var/www/tiphia-frontend/dist;
    index index.html;

    client_max_body_size 1m;

    location /api/ {
        proxy_pass http://127.0.0.1:7999;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Authorization $http_authorization;
        proxy_read_timeout 30s;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

注意事项：

- `location / { try_files ... /index.html; }` 是 SPA 必需配置，否则直接访问 `/admin`、`/posts/xxx` 会 404。
- `proxy_pass http://127.0.0.1:7999;` 里的端口应当是后端在宿主机暴露的端口，例如 `docker run -p 7999:3000`。
- `Authorization` 请求头必须转发，否则后台登录后的鉴权接口会出现 401。
- 如果前后端同源，后端 CORS 可以只填写这个前端来源；如果完全同源反代，浏览器不会触发跨域，但保留正确 CORS 仍然有利于排查和扩展。
- 静态资源更新后，如果使用 CDN 或浏览器强缓存，要清理缓存并确认页面加载的是新的 `dist/assets/app-*.js`。

如果你把 API 单独部署在 `https://api.example.com`，则不需要同源 `/api/` 反代，但后端 `cors.allowed_origins` 必须包含前端域名。

## 前端部署

前端仓库单独构建：

```bash
cd tiphia-frontend
yarn install
yarn build
```

构建结果在 `dist/`，可以交给 Nginx、对象存储、CDN、Cloudflare Pages、Vercel 等静态托管。

### API 地址配置方式

前端 API 地址有三种常见配置方式。

#### 方式一：同源反代，推荐

`.env` 或 `.env.production` 中留空：

```bash
VITE_TIPHIA_API_BASE=
VITE_TIPHIA_FRONTEND_BASE=/
```

构建后前端会请求：

```text
/api/v1/auth/status
/api/v1/geetest/config
/api/v1/plugins
```

这要求 Nginx 按上面的示例把 `/api/` 反代到后端。这个方式最不容易遇到 CORS 和浏览器本机 `127.0.0.1` 问题。

#### 方式二：构建时指定独立 API 域名

适合前端和后端不在同一域名：

```bash
VITE_TIPHIA_API_BASE=https://api.example.com yarn build
```

或者写入 `.env.production`：

```bash
VITE_TIPHIA_API_BASE=https://api.example.com
VITE_TIPHIA_FRONTEND_BASE=/
```

注意不要写末尾斜杠。后端配置中必须允许前端来源：

```toml
[cors]
allowed_origins = ["https://posts.example.com"]
```

#### 方式三：运行时覆盖，不重新构建

如果你希望同一份 `dist/` 在不同环境复用，可以在 `index.html` 的应用脚本前注入：

```html
<script>
  window.__TIPHIA_API_BASE__ = "https://api.example.com";
</script>
```

`window.__TIPHIA_API_BASE__` 的优先级高于构建时的 `VITE_TIPHIA_API_BASE`。如果留空或不设置，则使用同源请求。

### 前端部署到子路径

如果前端不是部署在域名根路径，而是部署在 `/blog/`、`/tiphia/` 这样的子路径，需要设置：

```bash
VITE_TIPHIA_FRONTEND_BASE=/blog/
```

并且 Nginx 要把该子路径也回退到对应的 `index.html`：

```nginx
location /blog/ {
    alias /var/www/tiphia-frontend/dist/;
    try_files $uri $uri/ /blog/index.html;
}
```

如果同源 API 仍然使用 `/api/`，`VITE_TIPHIA_API_BASE` 可以继续留空；如果 API 也在子路径，例如 `/blog-api/`，则需要显式设置 `VITE_TIPHIA_API_BASE=/blog-api`。

### 前端静态缓存策略

Vite 产物中的 `assets/app-*.js` 带 hash，可以长缓存；`index.html` 不建议长缓存，否则用户可能一直加载旧 JS：

```nginx
location = /index.html {
    add_header Cache-Control "no-cache";
}

location /assets/ {
    add_header Cache-Control "public, max-age=31536000, immutable";
}
```

每次发布后应全量覆盖 `dist/`，并清理 CDN 中的 `index.html`。如果控制台仍显示旧文件名，例如旧的 `app-Bh1UFQ3F.js`，说明浏览器或 CDN 仍在使用旧入口文件。
### 常见错误

如果浏览器控制台出现：

```text
GET http://127.0.0.1:3000/api/v1/auth/status net::ERR_CONNECTION_REFUSED
```

说明前端构建产物里仍然包含开发环境 API 地址。处理方式：

1. 检查 `.env`、`.env.production`、CI/CD 环境变量里是否还有 `VITE_TIPHIA_API_BASE=http://127.0.0.1:3000`。
2. 同源反代部署时把 `VITE_TIPHIA_API_BASE` 设为空。
3. 重新执行 `yarn build`。
4. 全量覆盖线上 `dist/`，并清理浏览器/CDN 缓存。
5. 在构建产物中搜索确认没有旧地址：

```bash
rg "127\.0\.0\.1:3000" dist
```
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