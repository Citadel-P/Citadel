# Docker Engine v1.49 protocol

`citadel-docker-api` contains disposable OpenAPI Generator output. It has no feature,
server, persistence, or Citadel adapter dependencies. The Docker adapter uses its
tag clients for finite REST calls, with handwritten negotiation and streaming.

## Regenerate and verify

From the repository root, with the pinned Rust toolchain, Java 17+ and curl installed:

```sh
cargo xtask docker-api
cargo xtask docker-api --check
cargo test --locked -p citadel-docker-api
```

`codegen/v1.49.yaml` is the Rust-owned Docker schema; no submodule is needed.
`codegen/source.json` pins its path and checksum, and the
OpenAPI Generator 7.25.0 JAR URL/checksum. The JAR is downloaded only by this explicit
xtask command and cached in `target/codegen`. Normal Cargo builds use committed
Rust files and do not run Java, download generators, or regenerate code.
`JAVA_HOME` selects Java; `CITADEL_OPENAPI_GENERATOR_JAR` can select a predownloaded
JAR, which must still match the pinned checksum.

The command verifies both inputs, applies the precondition-checked schema patches,
generates into a temporary directory, and formats with the pinned Rust toolchain.
Only `src/` and `Cargo.toml` are generated. They can be deleted and recreated using
the previously built `target/debug/xtask docker-api`; keep `codegen/`, tests and this
README. `--check` compares the complete generated file set without changing it.
Generator documentation, shell scripts, manifests and bookkeeping outside those
two outputs are discarded. CI runs regeneration verification and the fixture tests.

## Client ownership

The selected `reqwest-trait` configuration uses `topLevelApiClient=false`.
Construct the required tag clients with the same `Arc<apis::configuration::Configuration>`.
Set `client` to the caller's shared Unix-socket Reqwest client and explicitly provide
`base_path` (including the negotiated API version), authentication, user-agent,
request timeout and finite/error body bounds.
Do not use `Configuration::default()` in production: it constructs another client.

`tests/shared_client.rs` is a complete example. Two tag clients make version,
distribution/auth-header, and plain-text ping requests over the same accepted Unix
connection. The adapter retains responsibility for unversioned negotiation/ping,
error translation, cancellation, and streaming/hijack operations.

## Generation choices

All properties are in `codegen/generator.json`; no generated Rust is hand-edited.
`integer+uint64` and `integer+uint32` explicitly map to Rust unsigned types: the
stock generator otherwise maps these Docker formats to `i32`, overflowing ordinary
CPU/memory/network counters. Fixtures include counters exceeding both signed ranges.

The Cargo template shares pinned workspace dependencies, but pins Reqwest directly
to the same workspace version to avoid inheriting runtime-only TLS/stream features.
It enables only JSON and query support. `serde_with` needs only `std` for generated
double-option handling; its macros/base64/default features are disabled. Workspace
feature unification can still enable features required by the existing application.

The small `lib.mustache` override preserves the upstream exports and adds
`forbid(unsafe_code)` plus generated-code-only Clippy allowances for
`derivable_impls`, `empty_docs`, `into_iter_on_ref`, `manual_map`, and `needless_return`.
Upstream already allows `unused_imports` and `too_many_arguments`. These are template
style exceptions; reassess them on generator
upgrades. Handwritten tests, xtask and adapters retain normal lint rules.

See [compatibility.md](codegen/compatibility.md) for wire corrections, streaming boundaries and transport constraints. Generator properties are
documented in the [official Rust generator reference](https://openapi-generator.tech/docs/generators/rust/).

## Adapter integration

`adapters/src/connectors/docker/finite.rs` calls the generated traits for the operations Citadel
actually uses. One version-keyed `Arc<Configuration>` reuses the DockerClient's
Unix Reqwest pool; short-lived tag handles clone that Arc, never construct a pool.
`mapping.rs` normalizes optional protocol values into existing adapter snapshots in
`projection.rs`. Those snapshots preserve the adapter's semantic contracts; they
contain no HTTP endpoint catalog or transport methods.

The pinned reqwest-trait templates apply request timeouts and cap successful bodies
at 16 MiB and error bodies at 64 KiB before JSON decoding, including chunked bodies.
Body errors retain HTTP status so an HTTP 400 invalidates negotiation even when its
body is oversized or truncated. Missing Content-Type retains the old adapter's JSON
interpretation. Error text keeps the previous lossy UTF-8 behavior; successful JSON
uses strict UTF-8. Registry errors remain redacted by the adapter.

The model template retains unknown object fields using a flattened map. This is
needed for nested container configuration and Swarm read-modify-write operations.
The schema patches also preserve legacy defaults for descriptive image/volume/history
fields and correct observed nullable driver metadata. See the compatibility ledger.

To run the live fixture, provide a disposable Swarm daemon with the chosen image
already loaded (never the host daemon):

```sh
CITADEL_DOCKER_FIXTURE_TCP=127.0.0.1:52379 \
CITADEL_DOCKER_FIXTURE_IMAGE=alpine@sha256:85fe1e81d6758c208f3e1eed4338a1997e19d4be002d4dd32d3100c9a8c010a0 \
cargo test --locked -p citadel-adapters --test docker_live_transport -- --ignored
```

The fixture creates/deletes its own container, volume, network, service, secret and
config; it exercises finite operations and event/stat streams against real Docker.
Socket fixtures additionally cover logs, exec upgrade/resize, pull progress,
registry credentials, cancellation, body bounds, connection reuse and negotiation.
