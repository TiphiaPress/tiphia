# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS server-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY plugins ./plugins
COPY tools ./tools
COPY src ./src
RUN cargo build --release --locked --package tiphia --package tiphia-typecho-import

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=server-builder /app/target/release/tiphia /usr/local/bin/tiphia
COPY --from=server-builder /app/target/release/tiphia-typecho-import /usr/local/bin/tiphia-typecho-import
COPY tiphia.example.toml ./tiphia.example.toml
COPY README.md ./
COPY docs ./docs

ENV TIPHIA_CONFIG=/app/tiphia.toml \
    TIPHIA_ENV=production \
    TIPHIA_BIND=0.0.0.0:3000 \
    DATABASE_URL=sqlite:///app/data/tiphia.db?mode=rwc \
    TIPHIA_LOG_DIR=/app/logs

RUN useradd --system --home /app --shell /usr/sbin/nologin tiphia \
    && mkdir -p /app/data /app/logs \
    && cp /app/tiphia.example.toml /app/tiphia.toml \
    && chown -R tiphia:tiphia /app /usr/local/bin/tiphia /usr/local/bin/tiphia-typecho-import

EXPOSE 3000
VOLUME ["/app/data", "/app/logs"]

USER tiphia
CMD ["tiphia"]
