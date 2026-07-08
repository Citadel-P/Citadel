# Citadel Automation Actions

## Goal

Add Citadel Automation Actions: TypeScript scripts created in the Citadel UI, executed by Citadel Core with Deno, and audited through the normal Citadel activity model.

Actions are a free v1 feature. Do not add license gating.

An action can be:

- tested from the edit page
- run manually
- triggered by one cron schedule
- triggered by a webhook

Each trigger creates an action run with status, logs, code snapshot, arguments snapshot, and activity events.

## Product Surface

Menu:

```text
Automation -> Actions
```

Routes follow the existing Citadel resource shell:

```text
/automation/actions
/automation/actions/add
/automation/actions/edit/{id}
```

The action edit page uses the standard Citadel form layout:

- inline editable title and description
- header status indicator
- action buttons for run, test, and delete
- `Config`, `Runs`, and `Activities` tabs

## Non-Goals

Do not implement these in v1:

- visual workflow builder
- action-to-action chaining
- action marketplace
- raw shell execution
- terminal or exec support from actions
- agent-side execution
- multiple schedules per action
- separate action-runner service
- stored user access or refresh tokens
- secret injection into scripts
- arbitrary network access

## Domain Model

Use two domain entities:

- `AutomationAction`
- `ActionRun`

Tables:

- `actions`
- `actionruns`

There is no separate schedule table in v1. Each action owns one optional schedule.

### AutomationAction

Required fields:

- `Id`
- `Name`
- `Description`
- `Code`
- `CodeHash`
- `DefaultArgsJson`
- `Enabled`
- `ScheduleEnabled`
- `ScheduleCron`
- `ScheduleTimeZone`
- `Webhook`
- `TimeoutSeconds`
- `AlertOnFailure`
- `RunAsActorId`
- `CreatedByActorId`
- `CreatedAt`
- `UpdatedAt`
- `ControlState`
- `CurrentRunId`
- `LastScheduledRunAt`
- `RowVersion`

`DefaultArgsJson` must be a JSON object.

### ActionRun

Required fields:

- `Id`
- `ActionId`
- `ActionName`
- `Trigger`
- `Status`
- `RunAsActorId`
- `TriggeredByActorId`
- `ArgsJson`
- `CodeSnapshot`
- `CodeHash`
- `TimeoutSeconds`
- `QueuedAt`
- `StartedAt`
- `FinishedAt`
- `DurationMs`
- `ExitCode`
- `Logs`
- `ErrorMessage`

Run trigger enum:

```text
Manual
Test
Schedule
Webhook
```

Run status enum:

```text
Queued
Running
Succeeded
Failed
TimedOut
Cancelled
Rejected
```

## Configuration

Automation uses code defaults and does not require `.env` configuration for normal installs.

Expose only operational settings that an admin may reasonably tune:

```env
Automations__MaxParallelRuns=4
```

All automation settings bind to `AutomationOptions`, but most of them are internal defaults and should not be shown as required install configuration.

Advanced overrides:

- `Automations__Enabled`
- `Automations__DenoPath`
- `Automations__WorkDir`
- `Automations__DenoCacheDir`
- `Automations__InternalBaseUrl`
- `Automations__AllowNet`
- `Automations__DefaultTimeoutSeconds`
- `Automations__MaxTimeoutSeconds`
- `Automations__MaxLogBytes`
- `Automations__PollIntervalSeconds`
- `Automations__SchedulePollIntervalSeconds`

`Automations__AllowNet` is optional. When it is not configured, Citadel derives Deno's `--allow-net` target from `Automations__InternalBaseUrl`.

`install-server-deps.sh` installs Deno into the Core image and verifies it with `deno --version` during image build. The default `Automations__DenoPath=deno` resolves the executable from `PATH`.

## Runtime

Citadel Core executes action code by spawning Deno through the existing process execution abstraction. TypeScript is never executed inside the .NET process.

Execution flow:

1. A trigger creates an `ActionRun` with `Queued` status.
2. The run stores the action code snapshot and args snapshot.
3. Manual and test runs are claimed by the request handler and streamed back as JSON progress items.
4. Scheduled and webhook runs are claimed by the background worker.
5. Citadel builds a temporary TypeScript file in the configured work directory.
6. Citadel starts Deno with restricted permissions.
7. Citadel captures stdout and stderr into the run logs.
8. Citadel marks the run terminal: `Succeeded`, `Failed`, `TimedOut`, `Cancelled`, or `Rejected`.
9. Citadel emits activity events.

Use Deno with restricted permissions:

```text
deno run
  --no-prompt
  --allow-read=<run-dir>
  --allow-write=<run-dir>
  --allow-env=NO_COLOR,DENO_DIR
  --allow-net=<derived internal API host or Automations__AllowNet>
  <generated-action-file>.ts
```

Never use:

```text
deno run --allow-all
```

Do not grant subprocess, Docker socket, host environment, arbitrary file system, or arbitrary network access.

## Script API

Action code receives these globals:

```ts
citadel
args
run
console
```

`args` is the JSON object for the run.

`run` includes:

```ts
{
  id: string;
  actionId: string;
  actionName: string;
  trigger: "Manual" | "Test" | "Schedule" | "Webhook";
  queuedAt: string;
}
```

The helper exposes both raw HTTP methods and generated OpenAPI operations:

```ts
await citadel.request("GET", "/api/v1/deployments");
await citadel.get("/api/v1/deployments");
await citadel.post("/api/v1/stacks/apply", body);
await citadel.patch("/api/v1/...");
await citadel.put("/api/v1/...");
await citadel.delete("/api/v1/...");

await citadel.api.listPlatforms();
await citadel.platforms.listPlatforms();
await citadel.containers.getContainer(containerId);
await citadel.images.listImages(platformId);
await citadel.volumes.listVolumes(platformId);
await citadel.repositories.listGitRepositories();

await citadel.deployments.apply(input);
await citadel.stacks.apply(input);
await citadel.stacks.rollback(input);
```

`citadel.api` exposes automation-safe generated operation names. Tag groups such as `citadel.platforms`, `citadel.containers`, `citadel.images`, `citadel.volumes`, `citadel.gitRepositories`, and `citadel.repositories` expose the same generated operations grouped by OpenAPI tag.

The helper injects Citadel's internal base URL and a short-lived run token. Users do not configure base URLs or authorization headers in action code.

Monaco registers the generated frontend API sources from `src/Citadel.FrontEnd/src/api/generated/api.types.ts` and `src/Citadel.FrontEnd/src/api/generated/resources.ts` as virtual TypeScript modules, then declares the action globals from those generated types. This keeps `citadel`, `args`, `run`, and common Citadel API input/output types aligned with the OpenAPI-generated client. Backend authorization remains the security boundary.

## Authentication And Permissions

Actions run as `RunAsActorId`.

Defaults:

- on create, `RunAsActorId` defaults to the creator actor
- manual and test runs record the triggering actor
- schedule and webhook runs execute as the configured run-as actor

Citadel must not store or reuse user JWTs or refresh tokens.

When a run starts, Citadel creates a short-lived internal run token for the run identity. Backend endpoints still evaluate current Citadel permissions. If the run-as actor is disabled, deleted, or no longer authorized, the run fails clearly.

Resource type:

```text
AutomationAction
```

Capabilities follow existing Citadel permission levels:

- read actions and runs
- create actions
- update actions
- delete actions
- execute manual/test runs

Admin bypass follows the existing Citadel permission model.

## Endpoint Safety

Automation run tokens are blocked from unsafe endpoint groups server-side. Do not rely only on Monaco or frontend hiding.

Minimum blocked areas:

- authentication and session endpoints
- automation self-modification endpoints
- raw secret and secret-provider endpoints
- terminal endpoints
- exec endpoints

This prevents v1 actions from indirectly becoming shell execution.

## API

Use existing Minimal API and Mediator patterns.

Routes:

```text
GET    /api/v1/automation/actions
POST   /api/v1/automation/actions
GET    /api/v1/automation/actions/{id}
PATCH  /api/v1/automation/actions/{id}
DELETE /api/v1/automation/actions/{id}

POST   /api/v1/automation/actions/rename
PATCH  /api/v1/automation/actions/{id}/metadata

POST   /api/v1/automation/actions/{id}/run
POST   /api/v1/automation/actions/{id}/test
POST   /api/v1/automation/actions/{id}/runs/{runId}/cancel

GET    /api/v1/automation/actions/{id}/runs
GET    /api/v1/automation/actions/{id}/runs/{runId}
GET    /api/v1/automation/actions/{id}/runs/{runId}/logs

POST   /listener/{github|gitlab}/automation-action/{id}/run
```

The webhook listener is anonymous but protected by the configured provider authentication scheme and shared secret when a secret is configured.

## Queue And Concurrency

Queued runs are stored in the database.

The background worker:

- polls queued runs
- claims runs atomically
- respects `Automations__MaxParallelRuns`
- prevents the same action from running twice at the same time

Duplicate active-run behavior:

- manual/test: return a conflict and record a rejected run
- schedule/webhook: record a rejected run with a clear error message

If global capacity is full, runs remain queued.

## Scheduling

Each action can have one cron schedule:

- `ScheduleEnabled`
- `ScheduleCron`
- `ScheduleTimeZone`

The scheduler periodically evaluates enabled actions and queues due runs with `Trigger = Schedule`.

Missed schedules should not replay by default after downtime.

## Webhooks

When webhook triggering is enabled, actions use the same webhook config model and public listener as repositories and stacks:

- provider
- authentication scheme
- optional shared secret
- optional branch filter

Listener URL shape:

```text
/listener/{github|gitlab}/automation-action/{id}/run
```

Webhook payloads become action args. Object payloads are passed through as the run args object. Non-object payloads are wrapped:

```json
{
  "payload": "value"
}
```

The UI should make the secret visible only when it is first generated or rotated.

## Logs

Capture merged stdout and stderr.

Respect:

```text
Automations__MaxLogBytes
```

When the limit is reached, truncate logs and append a clear marker.

Redact obvious sensitive values before storing logs:

- bearer tokens
- action run tokens
- API keys
- password-like values
- secret-like values

## Activity Events

Activity resource type:

```text
AutomationAction
```

Lifecycle events:

```text
ActionCreated
ActionUpdated
ActionRenamed
ActionDeleted
```

Run events:

```text
ActionRunQueued
ActionRunStarted
ActionRunSucceeded
ActionRunFailed
ActionRunTimedOut
ActionRunCancelled
ActionRunRejected
```

The activity sheet should render:

- created/deleted snapshots in Monaco
- updated snapshots in Monaco diff
- renamed events as readable text
- run events with run id, trigger, duration, exit code, reason, and error message

## Validation

Action validation:

- name is required
- code is required
- description max length is enforced
- default args must be a JSON object
- timeout must be between 1 and `Automations__MaxTimeoutSeconds`
- cron is required when schedule is enabled
- timezone is required when schedule is enabled
- webhook secret length and branch filter length are bounded when configured

Run validation:

- automations must be enabled globally
- action must exist
- action must be enabled unless trigger is `Test`
- same action must not already be running
- run-as actor must be valid
- current permissions must allow requested backend operations

## Acceptance Criteria

- Users with permission can create, edit, rename, and delete actions.
- Action lifecycle mutations emit activity events.
- Users can test an action from the edit page.
- Users can run an enabled action manually.
- Deno executes TypeScript with restricted permissions.
- stdout and stderr are captured and visible in the Runs tab.
- Logs are capped and sensitive values are redacted.
- Same action cannot run twice at the same time.
- Timeout terminates the Deno process and records `TimedOut`.
- Scheduled actions create runs with `Trigger = Schedule`.
- Webhooks create runs with `Trigger = Webhook`.
- Actions run as the configured actor without storing user tokens.
- Automation run tokens cannot call blocked endpoint categories.
