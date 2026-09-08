# Container terminal parity check — 2026-09-07

## Confirmed defect

The .NET `ContainerService.ExecAsync` resize callback catches failures and keeps
the interactive shell open. Rust propagated a resize rejection from
`container_terminal::local_session`, terminating its output stream (and the
shared realtime connection propagated that stream error).

The new `terminal_resize_rejection_does_not_close_the_interactive_session` test
failed with `Docker terminal resize rejected` before the fix. Rust now logs
resize failures and retains the shell. Input/write failures remain errors;
cancellation still terminates the session. No frontend changes are required.

This is a measured failure scenario, not confirmation that the user's reported
session failed for this reason. The user's live API was unreachable during this
check. Normal Local Docker terminal use succeeded in the isolated fixture.

## .NET reference and coverage

- `ExecSessionManagerTests.StartExecProcess_RequiresAnExistingSubscription`:
  existing real-WebSocket test rejects Start before Join without an exec command.
- `ExecSessionManagerTests.RemovingLastSubscriber_CancelsAndDisposesExecSession`:
  Local test leaves the terminal group, observes Docker socket EOF and reconnects
  on the same WebSocket; transport test checks output disposal closes input.
- `ExecSessionManagerTests.StartExecProcess_ResourceId_RoutesToPersistedOwningNode`:
  existing PostgreSQL/WebSocket/Edge test resolves the UUID to the exact worker,
  rejects cross-connection input and cancels after permission revocation.
- `.NET ApplicationHub` invocation order and byte-array arguments: Local test
  sends Join, Start, initial Resize and browser-shaped Uint8Array JSON, verifies
  terminal output and repeats with UUID/short Docker IDs and bash/sh.
- `.NET ContainerService.ExecAsync` resize-error handling: rejected resize is
  tested at both Docker transport and Local WebSocket boundaries.

## Running

```sh
cargo test -p citadel-adapters --test docker_transport --test agent_mutations
# CITADEL_PHASE4_DATABASE_URL: disposable PostgreSQL database
cargo test -p citadel-server --test platforms_http -- --ignored --test-threads=1
```

The Local WebSocket test normally uses a Docker Unix-socket fixture which
deliberately rejects resize. For real Docker, set both
`CITADEL_TERMINAL_TEST_CONTAINER` (a disposable running container supporting
bash and sh) and `CITADEL_TERMINAL_TEST_SOCKET` before running
`local_terminal_browser_sequence_streams_output_and_reconnects`. The test opens
shells and executes a printf in that container. Never point it at a user workload.

Verification: 45 platform HTTP/WebSocket tests passed, including Local terminal
against real Docker on a disposable nginx container; 12 Docker transport and 5
signed Agent tests passed. This is not a full browser or live Edge acceptance run.
