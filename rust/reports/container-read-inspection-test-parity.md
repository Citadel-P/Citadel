# Container detail and inspection test parity

Reviewed against .NET on 2026-09-07. Test counts do not establish parity: each
scenario must exercise the same observable boundary. This is not whole-Citadel
parity certification.

## Source-to-Rust mapping

| .NET scenario | Rust equivalent | Boundary |
| --- | --- | --- |
| `GetContainerTests.Get_Container_ReturnsSuccess` | `platforms_http/get_container.rs::get_container_ports_the_complete_dotnet_summary_snapshot_without_deployment_secrets` | HTTP + PostgreSQL; every nonempty field of the verified snapshot including image/deployment summaries. Null/empty optional collections follow the .NET snapshot convention. Dates and IDs are fixture values. |
| `GetContainerTests.Get_Container_ByPersistedId_ReturnsSuccess` | `get_container_by_docker_and_persisted_id_returns_the_same_resource` and snapshot test | UUID/full/short Docker ID equality; authorization, invalid/missing IDs and ambiguity protection. |
| `ContainerInspectionRedactorTests.RedactEnvironment_ShouldMaskSensitiveValuesAndPreserveOrdinaryValues` | `adapters/tests/container_inspection.rs::inspection_masks_sensitive_environment_and_preserves_ordinary_values` | Unit; source environment entries, empty sensitive values and caller input preservation. |
| `RedactEnvironment_ShouldLeaveInspectionWithoutConfigurationUnchanged` | `inspection_without_configuration_preserves_other_fields` | Unit; complete normalized document equality. |
| `InspectStackContainer` permissions/ownership, used by Vault acceptance | `platforms_http/container_inspection.rs::inspection_resolves_ui_ids_enforces_inspect_permission_and_routes_to_the_owning_node` | HTTP/PostgreSQL. Stack Read alone cannot inspect; Platform Inspect cannot grant Stack Inspect. Stack Read + Inspect does not require additional Platform ACL. Wrong/unlinked containers never reach Docker. Both routes exercise the owning Edge session. |
| `GetContainersData` and Stack data -> returned ID -> inspect | Same HTTP test; `external_integrations/inspection.rs::verify` | Stack Read authorization, invalid/unauthorized requests, unlinking. Live test discovers Docker IDs through the public data route before inspection. |
| `ContainerSyncJob` ownership preservation/discovery | `platform_inventory_persistence.rs::inventory_restores_only_platform_scoped_ownership_and_preserves_adoption` | PostgreSQL. Only valid existing same-Platform ownership links resolve. Conflicting/malformed/orphaned/unmanaged labels cannot confer ownership. Existing adoption links survive conflicting snapshots. |
| `VaultKvV2CompatibilityTests.AssertInspectionIsRedactedAsync` within `VaultKvV2_ShouldValidateResolveInjectAndRedactSecret` | `external_integrations_acceptance.rs::forgejo_push_applies_git_stack_with_vault_secret_and_redacted_audit` | Real Vault/Forgejo/PostgreSQL/Docker. Production inventory persistence, login-issued bearer authentication, Stack data and both inspect routes after initial Apply and webhook replacement. Raw runtime injection is verified; API/progress/audit remain redacted. HTTP uses an in-process router, not a packaged Core/browser. |
| `RegularAgentCompatibilityTests.AssertRuntimeContainerIsRunningAsync` inspection fields | `agent_mutations.rs::signed_agent_inspection_preserves_image_state_and_redacts_secrets` | Signed gRPC fixture, not full live Core lifecycle. |
| Same Regular Agent fields and interactive transport | `agent_candidate_acceptance.rs::published_agent_authenticates_bounded_logs_and_interactive_terminal` | Executed against current published .NET Agent + disposable Docker workload. Signed inspection/redaction, wrong credentials, bounded logs and terminal input/output. Does not create a Stack through Core HTTP. |
| `EdgeAgentCompatibilityTests.AssertRuntimeContainerIsRunningAsync` inspection fields | `platforms_http/container_inspection.rs` | HTTP/PostgreSQL/Edge command-session fixture; exact node and no manager fallback. Not live Edge enrollment/Stack lifecycle. |

.NET sources: `test/Citadel.Tests.Integration/Application/Features.Containers/GetContainerTests.cs`,
its verified snapshot under `Snapshots`, `test/Citadel.Tests.Unit/Application/Services/ContainerInspectionRedactorTests.cs`,
and `test/Citadel.Tests.Acceptance/Compatibility/{RegularAgent,EdgeAgent,VaultKvV2}CompatibilityTests.cs`.
Behavior references: `src/Citadel.Application/Features.Stacks/Queries/{InspectStackContainer,GetContainersData}.cs`
and `TaskJobs/ContainerSyncJob.cs`.

## Bugs exposed and fixed

1. Detail accepted only UUIDs; the Docker-ID regression initially returned 400.
2. Stack inspection/data routes were missing. Restored .NET URLs and operation IDs without frontend workarounds.
3. The live test discovered a deployed container but no persisted Stack ownership. Reconciliation now restores valid same-Platform links while preserving adoption.
4. The detail response omitted image/deployment summaries. The snapshot is now ported, including an assertion that Platform-only readers cannot see Deployment configuration/bindings. Legacy summary defaults are preserved, not treated as full Deployment state.

Inspection shares redaction and Local/Agent/Edge dispatch, no-store responses,
bounded timeout and cancellation-on-drop. HTTP runtime data shares its mapper
with realtime messages; container queries share their SQL projection.

## Remaining gaps: full application parity is NOT complete

1. Packaged **Core + Regular Agent** lifecycle: platform create/handshake, Stack Apply,
   HTTP data/inspection and removal together. Passing an actual Agent adapter test
   and a Local external test does not replace that complete lifecycle.
2. Packaged **Core + live Edge Agent**: enrollment/reconnect, Stack Apply and public
   inspection against the actual Edge process. Session fixtures are not equivalent.
3. Other .NET mutation/reconciliation/stats/adoption and resource suites still need
   per-scenario audits. A passing platform suite does not certify all acceptance tests.
4. Browser acceptance is not covered here. See `container-terminal-parity.md` for
   the separately measured real-WebSocket/real-Docker terminal checks.

Do not mark these complete from counts or an ignored test merely compiling.

## Reproduction

Verified on 2026-09-07: 46 platform HTTP/WebSocket tests, 2 inventory-persistence
tests, 56 server unit tests, 7 inspection tests, 5 signed Agent tests and 12 Docker
transport tests passed. Live Vault/Forgejo/Docker acceptance and the current
published .NET Agent acceptance both passed. Generated contracts validated at
339 full / 287 public operations. All 9 realtime-subscription tests (including
the PostgreSQL-dependent test), formatting and strict Clippy checks passed.
These results do not close the lifecycle gaps
listed above.

Use a fresh disposable PostgreSQL database, never a populated product database.

```sh
cargo test -p citadel-server --test platforms_http -- --ignored --test-threads=1
cargo test -p citadel-adapters --test platform_inventory_persistence -- --ignored --test-threads=1
cargo test -p citadel-adapters --test container_inspection --test agent_mutations --test docker_transport
cargo clippy -p citadel-server -p citadel-adapters --tests -- -D warnings
cargo run -p xtask -- openapi --check
```

```powershell
./rust/scripts/Test-Phase7Integrations.ps1 -WorkspaceContainer <workspace>
./rust/scripts/Test-Phase7AgentCandidate.ps1 -DevContainer <workspace> -AgentImage <current-agent-image>
```

Live fixtures are uniquely named and cleaned individually. Existing workloads
and shared build-cache volumes are not pruned.
