# Build the frontend assets served by Core.
FROM node:24-alpine AS frontend-build
WORKDIR /source/frontend
COPY src/Citadel.FrontEnd/package.json src/Citadel.FrontEnd/package-lock.json ./
RUN npm ci
COPY src/Citadel.FrontEnd/ ./
ENV NODE_OPTIONS=--max-old-space-size=4096 \
    VITE_API_BASE_URL=""
RUN npm run build:image

FROM rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97 AS build

WORKDIR /source
ARG VERSION
ARG INFORMATIONAL_VERSION
ENV CITADEL_VERSION=${VERSION} \
    CITADEL_INFORMATIONAL_VERSION=${INFORMATIONAL_VERSION}
COPY version.json ./version.json
COPY rust ./rust
# build.rs compiles and verifies the shared Agent protobuf contracts.
COPY src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Protos \
    ./src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Protos
WORKDIR /source/rust
ENV SQLX_OFFLINE=true
RUN --mount=type=cache,id=citadel-rust-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=citadel-rust-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=citadel-rust-target,target=/source/rust/target \
    cargo build --locked --release -p citadel-server --bin citadel-server \
    && cp target/release/citadel-server /tmp/citadel-server

FROM docker.io/library/docker@sha256:f7d048590d889e00868920f658e807ecbc4ef662d51c9af67eb5534a447d58d6 AS docker-cli
FROM docker.io/denoland/deno@sha256:b0e1bf78aea23abe03edf5c411cb9db3e1011d8659bfdeafcb80c1f53cdd71d7 AS deno
FROM debian@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241 AS runtime-base

WORKDIR /app
ENV HOME=/app
ARG TARGETARCH
LABEL com.citadel.system="true" \
      com.citadel.system-role="core"

COPY --from=docker-cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker-cli /usr/local/libexec/docker/cli-plugins/ /usr/local/libexec/docker/cli-plugins/
COPY --from=deno /deno /usr/local/bin/deno
COPY rust/install-server-deps.sh .
RUN chmod +x install-server-deps.sh \
    && ./install-server-deps.sh \
    && rm install-server-deps.sh

RUN mkdir -p /app/data && chown -R 65532:0 /app
COPY --chmod=0755 rust/scripts/docker-entrypoint.sh /usr/local/bin/citadel-entrypoint
EXPOSE 8000 8001
HEALTHCHECK --interval=10s --timeout=3s --retries=3 --start-period=3s \
    CMD ["/usr/local/bin/citadel-entrypoint", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/citadel-entrypoint"]
CMD ["serve"]

# Development mounts the local Cargo binary into runtime-base, reusing its cache.
# The default production image still contains the release binary and built UI.
FROM runtime-base AS final
COPY --from=build --chown=65532:0 /tmp/citadel-server /app/citadel-server
COPY --from=frontend-build /source/frontend/dist /app/wwwroot
ENV CITADEL_RUST_STATIC_ROOT=/app/wwwroot
