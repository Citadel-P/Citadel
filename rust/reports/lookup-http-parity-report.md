# Resource lookup HTTP parity

Implemented on 2026-09-05. The missing `GET /api/v1/lookup` route is registered
in the Rust server and its shared contract metadata. It preserves the existing
SPA operation name, query parameters, and bare `ResourceInfo[]` response, so
dropdown callers continue using Citadel's existing `useRead` infrastructure.

## Behavior

- Accept `TargetResourceType`, optional `SourceResourceType`, `SourceResourceId`
  and `PlatformId` (also camelCase names). Invalid types/context return 400;
  unauthenticated requests return 401.
- Restrict identity/license lookups to administrators, including precedence over
  unsupported source/target combinations. Inaccessible sources return 404.
- Filter resource names using existing actor/team roles and resource overrides.
  Preserve referenced resources in edit-mode lookups without granting access to
  unrelated resources. Deduplicate linked and independently authorized results.
- Keep Deployment/Stack edit platform choices in their existing orchestration
  type; preserve the .NET add-mode rules.
- Return platform-scoped Images from persisted inventory. Networks/Volumes use
  existing Local/Agent transports, with the same unsupported Edge capability
  boundary as the other platform reads. There is no additional inventory cache.
- Return effective Global/resource binding names only, with case-insensitive
  resource overrides. Never select binding values or registry credentials.
- Return actor IDs for UserActor/RunAsActor. Service Account candidates require
  the license entitlement, enabled/unarchived accounts, and Read plus Use access.
- Reuse license instance-identity initialization; repeated License lookups return
  the same persisted identity.

## Tests and .NET references

Reference: `test/Citadel.Tests.Integration/Application/Features.Lookup/LookupTests.cs`
and `src/Citadel.Application/Features.Lookup/Queries/GetResourceLookupQuery.cs`.

`crates/server/tests/platforms_http/lookup.rs` exercises the real Axum route and
PostgreSQL store, plus a Docker HTTP fixture for Network/Volume lookups:

- Validation, administrator precedence, all supported standalone add targets,
  inaccessible source/scoped platform, current-user actor IDs and License ID.
- Deployment-linked Registry/Platform/Image selections, ACL filtering,
  linked/authorized deduplication and Platform-linked Deployment selections.
- Stack-linked Registry/Git/Platform selections and effective binding privacy.
- Run-as Read/Use, license and disabled-account gates.
- Team-role and team-override access, disabled teams, platform-type filtering,
  and administrator User-to-Team/Role lookups.
- Deployment and SwarmService scoped bindings, including inaccessible sources
  and case-insensitive overrides of Global bindings.

Unit tests cover lookup validation and route metadata. OpenAPI verification
compares the generated Rust operation with the existing frontend schema.
These are focused ports and additional regression cases, not a claim that the
entire .NET integration/acceptance suite has been migrated.

Run with a disposable migrated PostgreSQL database:

```sh
export CITADEL_PHASE4_DATABASE_URL=postgres://USER:PASSWORD@HOST/TEST_DATABASE
cargo test --locked -p citadel-resources --lib
cargo test --locked -p citadel-contracts --lib lookup
cargo test --locked -p citadel-server --test platforms_http -- --include-ignored --test-threads=1
cargo run --locked -p xtask -- openapi --check
```

The existing `scripts/Test-Phase4Reads.ps1` includes these HTTP tests. No database
schema or migration change is needed. Restart the Rust backend after rebuilding;
existing forms can call `/api/v1/lookup?TargetResourceType=Platform` immediately.

## Verification on 2026-09-05

- Resource unit suite: 8 passed (including 2 lookup cases).
- Lookup route contract test: 1 passed.
- Platform HTTP/PostgreSQL suite: 7 passed (including 6 lookup scenarios).
- Targeted server/library/binary/HTTP-test Clippy: passed with warnings denied.
- Generated OpenAPI/frontend compatibility check: passed.

Tests ran in the Linux devcontainer against an isolated PostgreSQL database,
which was removed afterward. The development database was not modified. The full
frontend and acceptance suites were not rerun for this backend lookup change.
