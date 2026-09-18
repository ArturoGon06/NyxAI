FROM rust:1.85-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY public ./public

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl \
    && groupadd --system nyxai \
    && useradd --system --gid nyxai --home-dir /app --shell /usr/sbin/nologin nyxai \
    && mkdir --parents /data \
    && chown nyxai:nyxai /data \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/nyxai /usr/local/bin/nyxai
COPY --chown=nyxai:nyxai --from=builder /app/public ./public

ENV NYXAI_PORT=8000
ENV NYXAI_DATABASE_URL=/data/nyxai.db
ENV NYXAI_ASSET_DIR=/data/avatars
ENV OLLAMA_BASE_URL=http://localhost:11434
ENV A1111_BASE_URL=http://localhost:7860
ENV NYXAI_IMAGE_DIR=/data/images

VOLUME ["/data"]
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl --fail --silent --show-error "http://127.0.0.1:${NYXAI_PORT:-8000}/api/health" || exit 1

USER nyxai
CMD ["nyxai"]
