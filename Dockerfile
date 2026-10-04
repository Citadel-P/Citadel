# Build the frontend assets served by Core.
FROM node:24-alpine AS frontend-build
WORKDIR /source/src/frontend
COPY src/frontend/package.json src/frontend/package-lock.json ./
RUN npm ci
COPY src/frontend/ ./
ENV NODE_OPTIONS=--max-old-space-size=4096 \
    VITE_API_BASE_URL=""
RUN npm run build:image

FROM rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97 AS rust-source

WORKDIR /source
ARG VERSION
ARG INFORMATIONAL_VERSION
ENV CITADEL_VERSION=${VERSION} \
    CITADEL_INFORMATIONAL_VERSION=${INFORMATIONAL_VERSION}
COPY version.json ./version.json
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo/ ./.cargo/
COPY src/.sqlx/ ./src/.sqlx/
COPY src/server/ ./src/server/
COPY src/agent/ ./src/agent/
COPY src/features/ ./src/features/
COPY src/infrastructure/ ./src/infrastructure/
COPY src/tools/build/ ./src/tools/build/
COPY src/tools/xtask/ ./src/tools/xtask/
WORKDIR /source
ENV SQLX_OFFLINE=true

FROM rust-source AS volume-helper-build
# Cargo's package-cache lock lives outside these shared registry/git directories.
RUN --mount=type=cache,id=citadel-rust-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=citadel-rust-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=citadel-volume-helper-target,target=/source/target,sharing=locked \
    cargo build --locked --release -p citadel-volume-helper \
    && cp target/release/citadel-volume-helper /tmp/citadel-volume-helper

FROM rust-source AS build
RUN --mount=type=cache,id=citadel-rust-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=citadel-rust-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=citadel-rust-target,target=/source/target,sharing=locked \
    cargo build --locked --release -p citadel-server --bin citadel-server \
    && cp target/release/citadel-server /tmp/citadel-server

FROM docker.io/library/docker@sha256:f7d048590d889e00868920f658e807ecbc4ef662d51c9af67eb5534a447d58d6 AS docker-cli
# Strip tools before copying them so the runtime never stores their larger originals.
RUN apk add --no-cache binutils \
    && strip --strip-unneeded /usr/local/bin/docker \
        /usr/local/libexec/docker/cli-plugins/docker-compose

# Deno 2.9.6 on Alpine 3.24 includes isolated glibc libraries for Deno and Core.
# Keep the GNU Rust build compatible with locally built development binaries.
FROM docker.io/denoland/deno@sha256:aa665f8777136863b5b8a0445a5cdfccff8103b5f40c9a877de5276b04facb1e AS runtime-base

WORKDIR /app
ENV HOME=/app \
    DENO_DIR=/app/.cache/deno
ARG TARGETARCH
LABEL com.citadel.system="true" \
      com.citadel.system-role="core"

COPY --from=docker-cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker-cli /usr/local/libexec/docker/cli-plugins/ /usr/local/libexec/docker/cli-plugins/
COPY src/tools/container/install-server-deps.sh .
RUN chmod +x install-server-deps.sh \
    && ./install-server-deps.sh \
    && rm install-server-deps.sh

RUN adduser -D -H -u 65532 -G root -h /app citadel \
    && mkdir -p /app/data && chown -R 65532:0 /app
COPY --from=volume-helper-build /tmp/citadel-volume-helper /usr/local/bin/citadel-volume-helper
COPY --chmod=0755 src/tools/container/docker-entrypoint.sh /usr/local/bin/citadel-entrypoint
EXPOSE 8000 8001
HEALTHCHECK --interval=10s --timeout=3s --retries=3 --start-period=3s \
    CMD ["/usr/local/bin/citadel-entrypoint", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/citadel-entrypoint"]
CMD ["serve"]

# Development stages the locally built binary and mounts it into runtime-base.
# The default production image still contains the release binary and built UI.
FROM runtime-base AS final
COPY --from=build --chown=65532:0 /tmp/citadel-server /app/citadel-server
COPY --from=frontend-build /source/src/frontend/dist /app/wwwroot
ENV CITADEL_RUST_STATIC_ROOT=/app/wwwroot
