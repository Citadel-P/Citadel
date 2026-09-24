# Docker v1.49 compatibility ledger

The authoritative source is `codegen/v1.49.yaml`, with reviewed wire corrections
in `codegen/patches/wire.json`. The cases below preserve the historical client
behavior and the Rust adapter's null-collection fixtures.

| Historical issue / endpoint | Schema and generated Rust result | Applies? / chosen handling | Evidence / owner |
|---|---|---|---|
| `Task` renamed `Task1` | Rust generates `models::Task` without a standard-library name collision. | No rename needed. | `task_name_and_unknown_response_properties_are_compatible`; generated crate |
| SystemEvents needs Stream | The schema names EventMessage, but the method buffers the complete body. | Yes; generated method is unsuitable for incremental events. | `apis/system_api.rs`; Phase 4 adapter |
| ImageCreate needs Stream | Generated method buffers pull/import progress. | Yes; retain incremental progress/error parsing. | `apis/image_api.rs`; Phase 4 adapter |
| ImageGet / ImageGetAll export | Binary tar responses are not finite JSON models; generated methods consume the body. | Yes; retain raw byte streaming. | `apis/image_api.rs`; Phase 4 adapter |
| ServiceLogs / TaskLogs | Log streams are multiplexed or raw depending on TTY; generated methods buffer. | Yes; retain framing, cancellation and streaming. | `apis/service_api.rs`, `task_api.rs`; Phase 4 adapter |
| ExecStart needs Stream | Docker can upgrade/hijack the connection for interactive exec; buffering cannot implement duplex I/O. | Yes, stronger than a return-type change. | `apis/exec_api.rs`; Phase 4 adapter |
| DistributionInspect lacks X-Registry-Auth | v1.49 lists only the path parameter; stock signature has no auth header. | Yes; append the optional header parameter in `patches/wire.json`. | `shared_client` checks the transmitted header; generated crate |
| Missing NetworkStats | This pinned schema actually defines `ContainerNetworkStats` including endpoint/instance IDs. Its `networks` map lacks `type: object`, producing untyped JSON. | Different; add the map's object type, reuse the existing model. No extension model. | `statistics_preserve_unsigned_counters_above_i32_and_i64`; generated crate |
| Per-property JsonStringEnumConverter removal | Rust derives serde and serde_repr; there is no C# converter-precedence mechanism. | No equivalent workaround. String volume scope and numeric filesystem change serialize correctly. | volume/numeric-enum fixtures; generated crate |
| Anonymous empty types replaced | Empty-object maps generate `HashMap<String, serde_json::Value>`. | Different; `{}` port/volume entries round-trip without a new model. | `optional_collections_and_empty_object_maps_round_trip`; generated crate |
| Nullable VolumeScope | Required Scope is marked non-nullable in Swagger, but existing compatibility behavior accepts null. | Yes; patch `x-nullable` to true, generate `Option<Scope>`. | `volume_nullability_matches_daemon_payloads`; generated crate |
| SecretCreate / ConfigCreate uppercase ID | This schema already references SwarmResourceCreateResponse for both; generated serde uses `ID`. | Already correct; no patch. | uppercase-ID fixture and both API signatures; generated crate |
| Rust null-to-empty collections | Most optional collections already deserialize null as None (or Some(None) for explicit nullable fields). Required image tags/digests/labels and volume labels/options reject null in stock output. | Patch only those required collection properties as nullable. Keep None in protocol models; Phase 4 maps to empty where the existing adapter requires it. | image/volume/network/config/system fixtures; generated crate + Phase 4 mapping |
| Unsigned Docker counters | Swagger uses uint64/uint32; stock Rust output uses i32. | Explicit generator type mappings preserve unsigned widths, including nested arrays/maps. No schema edit. | large-counter fixture; generated crate |
| Additional response properties | Docker permits future fields; generated serde structs have no deny_unknown_fields. | Preserve unknown fields in flattened maps, including nested mutation specs. Unknown enum values still fail decoding. | Task/SystemInfo/stats fixtures; generated crate |
| ContainerCreate schema composition | The generator warns about some allOf constructs. ContainerCreate must retain inherited container configuration and local host/network fields. | The representative nested payload round-trips correctly; no schema patch. | `container_create_preserves_inherited_configuration_and_host_options`; generated crate |

`patches/wire.json` is an ordered list of JSON-pointer additions/replacements over
the parsed pinned YAML. Replacements require an exact `expected` value; additions
require an absent key. Missing targets, changed source bytes, or reapplying a patch
fail generation. JSON is only the temporary normalized input; the pinned YAML stays
unchanged. No generated `.rs` post-processing occurs beyond rustfmt.

## Streaming and raw-operation boundary

Generating an operation does not authorize using it for every transport mode.
The existing adapter remains the runtime owner of SystemEvents, ImageCreate,
ImagePush, ImageBuild, ImageLoad, ImageGet, ImageGetAll, ContainerLogs, ServiceLogs,
TaskLogs, ContainerAttach, ContainerAttachWebsocket, ContainerExport,
ContainerArchive, PutContainerArchive, ExecStart, and Session. These involve progress
streams, raw archives, multiplexing, websocket or upgraded duplex connections.
ContainerStats can use a generated response model for a finite snapshot, but its
streaming mode must retain the handwritten transport. Large finite archives must
also remain streamed; “finite” alone does not make buffering appropriate.

The shared-client fixture proves that ordinary finite JSON and text/plain responses
work with injection. Phase 4 connects the production adapter to these clients and adds endpoint-specific
transport fixtures.

## topLevelApiClient comparison

Both shapes were generated from the identical pinned v1.49 input with Generator
7.25.0 and reqwest-trait. Models, method signatures, buffered-body behavior and
mockable API traits are identical. With `false`, each tag client holds one Arc to
Configuration. With `true`, ApiClient eagerly constructs 15 boxed trait objects,
each holding a clone of that same Arc; it adds no extra connection pools itself.

Choose **false**: the adapter can construct only required tag handles, error/model
mapping is unchanged, tests can still substitute per-tag traits, and ownership is
visible without an aggregate object's allocations. Both retain async_trait's boxed
future boundary. Neither shape removes the need for handwritten streaming.

An isolated Linux x86_64 construction smoke test used the pinned Rust toolchain and
workspace release settings (opt-level z, thin LTO, one codegen unit, abort, stripped).
Each binary explicitly built Configuration around a Reqwest client, constructed
either SystemApiClient or ApiClient, and passed it through `black_box`. Dependencies
were warm; these single compile timings ran alongside workspace validation and are
not a stable performance benchmark.

| Measurement | false / SystemApiClient | true / aggregate ApiClient |
|---|---:|---:|
| `apis/mod.rs` bytes before rustfmt | 3,580 | 8,218 |
| Inline client size | 8 bytes | 240 bytes, plus 15 heap boxes |
| Configuration Arc references including caller | 2 | 16 |
| Stripped construction-test executable | 814,600 bytes | 2,835,328 bytes |
| Warm-dependency release build | 56.39 s | 62.89 s |
| Compiler peak RSS | 898,840 KiB | 903,860 KiB |

This measures the generated boundary in isolation, not the Citadel server binary
or request latency. Actual runtime impact is deferred until adapter migration.
The spike uses JSON booleans for `topLevelApiClient`; a string-valued CLI property
can be treated as truthy by the template even when its text is `false`.

## Phase 4 production operation classification

This covers all 60 operations in the Phase 0 inventory. The schema still generates
unused operations, but the adapter does not integrate them speculatively.

| Operations | Production path |
|---|---|
| SystemInfo, SystemDataUsage | Generated; storage enrichment and the existing cache remain adapter semantics |
| SystemVersion, SystemPing | Handwritten bounded unversioned negotiation/health handshake |
| ContainerList, ContainerCreate, ContainerDelete, ContainerStart, ContainerStop, ContainerRestart, ContainerPause, ContainerUnpause | Generated; state changes still accept idempotent HTTP 304 |
| ContainerInspect, ImageInspect | Generated for typed semantic reads; bounded raw JSON for UI inspection documents |
| ContainerStats | Generated for finite snapshots; handwritten incremental stream with the existing frame projection |
| ContainerLogs, ServiceLogs, SystemEvents, ImageCreate | Handwritten incremental framing, cancellation and 1 MiB line bounds |
| ContainerExec, ExecInspect, ExecResize | Generated; resize failure remains best-effort within the terminal session |
| ExecStart | Handwritten HTTP upgrade and duplex transport |
| ImageList, ImageHistory, ImageDelete, DistributionInspect | Generated; registry auth is scoped to the call and registry errors are redacted |
| VolumeList, VolumeInspect, VolumeCreate, VolumeDelete | Generated; missing UsageData still joins the volume-only system/df result by name |
| NetworkList, NetworkInspect, NetworkCreate, NetworkDelete | Generated |
| VolumePrune, NetworkPrune, ImagePrune, BuildPrune | Generated with the existing filters |
| SwarmInspect, NodeList, NodeInspect, NodeUpdate | Generated |
| ServiceList, ServiceInspect, ServiceCreate, ServiceUpdate, ServiceDelete | Generated; nested unknown spec fields survive read-modify-write |
| TaskList, TaskInspect | Generated; existing service/node/running-task filters retained |
| SecretList, SecretInspect, SecretCreate, SecretUpdate, SecretDelete | Generated |
| ConfigList, ConfigInspect, ConfigCreate, ConfigUpdate, ConfigDelete | Generated |

Container/image raw inspect stays separate because the UI promises the complete
original document, including unknown fields and exact absent/null representations.
The generated models' open-field map protects mutations but does not promise a
byte-for-byte or absent/null-preserving representation of every known field.
Stream event/progress projections retain their tolerant legacy decoding; transport
never buffers an indefinite generated operation.

Additional compatibility findings in migration:

- ServiceUpdate's version parameter had no format and generated i32. Its patch now
  generates i64, matching existing optimistic-concurrency versions; a 5-billion
  version round-trips in the socket fixture.
- Required descriptive image/volume/history fields were defaulted by the old
  adapter. Targeted `x-citadel-default` properties retain that behavior while IDs
  remain required. Nullable history tags and create warnings are accepted.
- Missing volume usage size stays unknown, rather than becoming zero; incomplete
  usage remains excluded from aggregated totals.
- DistributionInspect may omit Platforms; Descriptor remains required.
- Docker's containerd image store emits `GraphDriver.Data: null`. DriverData.Data
  is patched nullable and covered by a regression fixture plus the real daemon.
- Query parameter ordering can differ (terminal resize sends h before w); values
  and Docker semantics are unchanged. Generated known enums now reject unknown
  enum strings; the flattened maps preserve unknown object properties, not enums.
- Finite stats parse schema timestamps as RFC 3339 and normalize their textual
  representation at the adapter boundary. Unsigned counters retain their width.

Two function-local `clippy::too_many_arguments` exemptions retain the existing
`resource_health` and `event_consumer` signatures in `server/src/workers/platforms.rs`.
Phase 5 must group their worker dependencies and remove these exemptions. Runtime
worker restructuring is explicitly outside Phase 4. Existing generated-only
allowances remain confined to the protocol crate and its pinned lib template;
remove each when an upstream generator/template update eliminates its cause.

## Phase 4 release and idle comparison (2026-09-19)

Measured the native Linux release server with the Phase 0 pinned PostgreSQL,
Docker-in-Docker and Alpine digests, the same clean environment, one administrator,
one Local platform, no workload containers, DB pool limit 5, ten-second monitoring,
and the same Unix proxy and one-second process sampler. The daemon was recreated
after the live Swarm fixture. The first 60 seconds include startup/registration;
the following 300 idle samples match Phase 0's `idle-5m` window. No docs/realtime
traffic or later mixed-workload soak is included in this comparison.

| Metric | Phase 0 | Phase 4 | Delta |
|---|---:|---:|---:|
| Release binary, bytes | 39,022,576 | 40,731,584 | +1,709,008 (+4.38%) |
| Idle mean RSS, MiB | 106.67 | 107.62 | +0.95 |
| Idle mean PSS, MiB | 104.30 | 105.50 | +1.20 |
| Sampled startup peak RSS, MiB | 106.39 | 107.09 | +0.71 |

The final-build capture showed a small idle increase; the log contained no Docker errors.
This is a cumulative Phase 0-to-4 regression check, not an isolated attribution to
code generation or an optimization claim. The host was shared with other services
and validation builds. Startup peak is the maximum one-second RSS sample, not the
kernel's exact VmHWM. The fifteen-minute idle and thirty-minute mixed-workload
windows were not repeated in Phase 4. Phase 5 must take its own workload baseline
before tuning. No memory/worker scheduling changes were made for this measurement.
Raw evidence is outside Git under
`~/.codex/citadel-refactor-audit-20260919/refactor-v13/phase4-idle/`.

## Migration cleanup and remaining boundaries

The unused `adapters/src/docker/generated.rs` and its `schema/source.json` are
removed. `xtask/src/main.rs` no longer contains `OPERATIONS`, `MODEL_FIELDS`, the
subset renderer, or the `docker` subcommand. CI verifies `docker-api --check` only.
The replacement adapter files are `finite.rs`, `mapping.rs`, and `projection.rs`;
existing inventory, mutation, image, storage and runtime modules retain their roles.
Snapshots in `projection.rs` normalize the generated models for those existing
semantic callers; they contain no endpoint inventory or HTTP dispatch machinery.

Phase 4 keeps the bounded raw inspection/stream/upgrade code described above.
Runtime scheduling, worker dependency grouping, retained-memory tuning, resource
views and broader Deployment/Stack/Swarm service organization remain later work.
Workspace formatting and mechanical lint fixes (including placing helpers before
test modules) were needed to make the mandatory checks pass; these do not change
worker scheduling or API behavior. The HTTP fixtures now include the pre-existing
visible-container query during get_info/initial inventory; the terminal fixture
accepts the generated h/w query order. No audit scripts or measurement data are
added to the repository.

## Completed validation

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed with
  only the two documented function-local worker arity exemptions above.
- `cargo test --locked --workspace`: 629 passed, 233 fixture-gated tests ignored.
- `cargo run --locked -p xtask -- openapi --check`: 404 full / 305 public operations verified.
- `cargo run --locked -p xtask -- docker-api --check`: all 249 generated files verified.
- `cargo test --locked -p citadel-docker-api`: 10 passed.
- Adapter socket fixtures: 17 Docker transport and 2 Swarm transport tests passed.
- Disposable real Docker/Swarm fixture: passed, including finite CRUD and event/stat streams.
- `platforms_http --ignored --test-threads=1` against the disposable PostgreSQL fixture:
  all 62 passed, including node coverage, container counts, authorization and terminal resize.
- Release build, matched startup/five-minute idle capture and `git diff --check`: passed.

The default workspace ignore count includes fixtures run explicitly above; other
external integration fixtures were not enabled. Disposable Phase 4 Docker and
PostgreSQL containers were removed after verification. The measured release binary
SHA-256 is `b9ddfb3a3dd9c4f35f9924cd822d4940e93463f5a04c35c6dc7408b7d3a7d88a`.

## Unpublished container ports

Docker represents an exposed port without a host binding as a null map value,
for example `NetworkSettings.Ports = {"6379/tcp": null}`. The pinned schema already
marks `PortMap.additionalProperties` nullable, but the stock Rust datatype rendering
loses the optional wrapper around these array values. The model template preserves
nullable array-valued maps as `HashMap<String, Option<Vec<T>>>`, including both
`NetworkSettings.Ports` and `HostConfig.PortBindings`. Null, empty and populated
bindings remain distinct during round-trip serialization. This prevents a successful
container start from failing its subsequent inspection and leaving a processing claim.
The wire-model and generated-transport tests cover the Redis-shaped response without
requiring a live daemon. The generator templates also retain the workspace lint
inheritance and import layout of the accepted generated code.
