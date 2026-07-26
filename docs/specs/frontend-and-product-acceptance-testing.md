# Citadel Frontend and Product Acceptance Testing

This is the implementation specification for automated frontend testing and
product-level acceptance testing in Citadel.

Before implementing this specification:

1. Keep the existing xUnit v3, `WebApplicationFactory`, Testcontainers,
   React, Vite, React Query, generated API client, and GitHub Actions patterns.
2. Do not attempt to validate every external integration through the browser.
   Test protocol and data integrity at the lowest layer that provides the
   required confidence.
3. Keep tests deterministic, isolated, cross-platform where practical, and
   runnable by a developer without access to shared infrastructure.
4. Do not add production test bypasses, authentication backdoors, or public
   test-data endpoints.
5. Pin test dependency and container image versions. Do not use `latest` tags
   in automated test environments.

---

## 1. Purpose

Citadel has backend unit and integration tests, but its React frontend has no
automated tests. Critical behavior currently depends on manual verification,
including authentication bootstrap, refresh-token recovery, authorization,
resource forms, React Query cache updates, SignalR events, Docker operations,
Git stack releases, backups, external secrets, webhooks, and Edge Agent
enrollment.

The test system defined here must:

- catch frontend regressions quickly during development and pull requests;
- exercise critical user journeys in a real browser;
- verify Citadel-owned integration behavior against real compatible services;
- avoid making every browser test depend on every external service;
- provide enough failure evidence to diagnose CI failures without reproducing
  them locally first;
- validate the release candidate before it is published as a stable image.

This specification defines the test architecture and initial mandatory test
portfolio. It does not require exhaustive coverage of every component or
resource in the first implementation slice.

## 2. Current Citadel Alignment

The implementation must build on the following current structure:

- `src/Citadel.FrontEnd` is a React 19 and Vite application.
- The frontend uses React Router, React Query, a generated API client, Radix UI,
  Monaco, xterm, and SignalR.
- `test/Citadel.Tests.Unit` contains backend unit tests.
- `test/Citadel.Tests.Integration` uses xUnit v3,
  `WebApplicationFactory<Program>`, Testcontainers for PostgreSQL, and an
  isolated database per test class.
- `.github/workflows/docker-publish.yml` currently builds the application and
  runs backend unit and integration tests.
- The application Dockerfile runs the frontend TypeScript build and lint, but
  it does not run frontend behavior or browser tests.
- The production container serves the compiled frontend and the API from the
  same origin.

The new system must not replace the existing backend suites. It adds frontend,
browser, and real-service acceptance coverage around them.

## 3. Goals

- Provide a fast frontend test command suitable for watch mode.
- Test user-observable behavior instead of React implementation details.
- Test API loading, success, validation, authorization, conflict, and failure
  states without starting the backend.
- Test authentication bootstrap and refresh-token behavior explicitly.
- Test SignalR-driven state changes and listener lifecycle explicitly.
- Run a small, stable Chromium smoke suite on every pull request.
- Run real Keycloak, Vault-compatible KV v2, RustFS S3-compatible storage,
  Forgejo, Docker, and Edge Agent acceptance suites on a schedule.
- Validate stack deployment and rollback as core Citadel behavior.
- Validate interruption recovery, idempotency, concurrency, and stale-operation
  protection for long-running work.
- Validate upgrades from the previous release and restoration of Citadel's own
  control-plane data before stable publication.
- Prove authorization at API, lookup, search, and SignalR delivery boundaries.
- Prove license capability enforcement, transitions, and direct-request
  resistance.
- Store traces, screenshots, browser logs, network failures, and service logs
  for failed browser and acceptance runs.
- Make every test independently repeatable and safe to rerun.
- Add a regression test for every production frontend bug that can be
  reproduced automatically.

## 4. Non-Goals

- Testing React, React Query, Radix UI, shadcn components, Monaco, xterm, or
  Playwright themselves.
- Achieving an arbitrary global line-coverage percentage.
- Running every browser engine and every external integration on every commit.
- Recreating all backend unit and integration assertions through the UI.
- Testing public GitHub, cloud S3, or a production identity provider from CI.
- Building a general-purpose test orchestration platform.
- Making tests order-dependent or preserving state between CI runs.
- Adding a production API that creates users, tokens, permissions, or resources
  specifically for tests.

## 5. Test Taxonomy

Citadel will use five complementary test layers.

| Layer | Runner | Dependencies | Primary purpose |
| --- | --- | --- | --- |
| Static checks | TypeScript, ESLint, Prettier | None | Compile-time and formatting failures |
| Frontend behavior | Vitest and React Testing Library | MSW only | Components, hooks, providers, routing, cache behavior |
| Backend integration | Existing xUnit project | PostgreSQL | API, persistence, authorization, migrations |
| Browser smoke | Playwright | Citadel, PostgreSQL, Docker | Critical end-to-end user journeys |
| Product acceptance | xUnit and selected Playwright tests | Candidate images and real services | Recovery, upgrades, compatibility, and data integrity |

Tests must be placed at the lowest layer that can prove the required behavior.
For example:

- form validation belongs in a frontend behavior test;
- API authorization belongs in a backend integration test;
- refresh-cookie recovery after a browser reload belongs in Playwright;
- backup byte integrity against S3 belongs in a .NET acceptance test;
- configuring and starting an S3 backup through the UI needs only one browser
  acceptance journey.

## 6. Tooling Decisions

### 6.1 Frontend Behavior Tests

Add these development dependencies to `src/Citadel.FrontEnd/package.json`:

- `vitest`;
- `@vitest/coverage-v8`;
- `jsdom`;
- `@testing-library/react`;
- `@testing-library/user-event`;
- `@testing-library/jest-dom`;
- `msw`.

Use Vitest with `jsdom` for the first implementation slice. Real-browser
component testing may be added later for components that cannot be exercised
reliably under `jsdom`, but it must not duplicate Playwright end-to-end tests.

Add these scripts:

```json
{
  "test": "vitest",
  "test:run": "vitest run",
  "test:coverage": "vitest run --coverage"
}
```

### 6.2 Browser Tests

Create a separate Node project under `test/Citadel.Tests.E2E` using:

- `@playwright/test`;
- `@axe-core/playwright` for focused accessibility checks.

The E2E project must have its own `package.json` and lock file. It must not
depend on `node_modules` from `src/Citadel.FrontEnd`.

### 6.3 Product Acceptance Tests

Create `test/Citadel.Tests.Acceptance` as an xUnit v3 executable test project.
Reuse versions and conventions from `Citadel.Tests.Integration`.

Add generic Testcontainers support as needed. Prefer purpose-built
Testcontainers modules when a maintained module exists; otherwise use
`ContainerBuilder` with explicit:

- image version;
- network alias;
- exposed ports;
- environment;
- disposable volume or bind-mounted fixture;
- HTTP or container health readiness check;
- startup timeout;
- captured logs.

## 7. Repository Layout

Use this structure:

```text
src/Citadel.FrontEnd/
  src/
    test/
      setup.ts
      render-citadel.tsx
      server.ts
      handlers/
      factories/
    **/*.test.ts
    **/*.test.tsx

test/
  Citadel.Tests.Acceptance/
    Authentication/
    Authorization/
    Backups/
    EdgeAgents/
    Git/
    Licensing/
    Recovery/
    Scheduling/
    Secrets/
    Upgrades/
    Infrastructure/
  Citadel.Tests.E2E/
    package.json
    package-lock.json
    playwright.config.ts
    compose.yml
    fixtures/
    tests/
      smoke/
      acceptance/
    support/
```

Co-locate frontend behavior tests with the production feature when the test is
specific to that feature. Keep shared test infrastructure under
`src/test`.

Do not place generated API files, shadcn primitives, or static icon mappings
under coverage enforcement.

## 8. Frontend Test Harness

Create `renderCitadel` as the only shared rendering entry point for feature
tests. It must support:

- a fresh React Query client per test;
- retries disabled;
- a memory router with a configurable initial route;
- the generated API client pointed at MSW;
- authentication state selection;
- optional SignalR test connection injection;
- theme and required application contexts;
- access to `userEvent`;
- cleanup after every test.

The helper must return Testing Library queries and the created Query Client.
Tests must not share a Query Client.

Use accessible queries in this priority:

1. role and accessible name;
2. label;
3. visible text;
4. stable semantic test ID only when the element has no practical accessible
   selector.

Do not assert Tailwind class lists or private component state. Assert visible
behavior, enabled state, navigation, requests, cache output, and focus.

### 8.1 Browser API Polyfills

The shared setup may provide minimal deterministic implementations of browser
APIs absent from `jsdom`, including:

- `ResizeObserver`;
- `IntersectionObserver`;
- `matchMedia`;
- `scrollIntoView`;
- `requestAnimationFrame`.

Do not create broad mocks that hide application bugs. Monaco, xterm, canvas,
and layout-sensitive behavior must be covered by focused adapters or
Playwright rather than increasingly complex DOM emulation.

## 9. API Mocking

Use MSW at the network boundary. Frontend tests must exercise the generated API
client rather than mocking each React Query hook.

Create default handlers for stable application bootstrap requests. Tests may
override only the endpoint relevant to the scenario.

Provide typed response factories for common models, including:

- current profile;
- license;
- platform;
- stack and stack release;
- deployment;
- alert event;
- activity;
- role and permission;
- backup policy and repository;
- Git repository;
- build and build pool.

Factories must produce the smallest valid object and allow explicit overrides.
Avoid one large fixture graph shared by unrelated tests.

The default MSW policy is strict:

- an unhandled API request fails the test;
- handlers must validate important request bodies and route parameters;
- tests must assert mutations when behavior depends on their payload;
- generated API request and response types must be used where available.

## 10. SignalR Testability

Do not mock `@microsoft/signalr` independently in every test.

Introduce a small application-owned SignalR connection factory or adapter that
the existing SignalR provider consumes. Production creates a real
`HubConnection`; tests create a deterministic fake connection.

The fake must support:

- connection state transitions;
- `start`, `stop`, and reconnect callbacks;
- `on` and `off`;
- `invoke`;
- emitting a named server event;
- recording group joins and leaves;
- detecting duplicate listeners.

Frontend tests must cover:

- a server event updates the relevant React Query cache;
- listeners are removed when a resource or tab unmounts;
- remounting does not create duplicate listeners;
- reconnect rejoins active groups;
- an event for an inaccessible or unrelated resource is ignored;
- an event received during a list refetch does not corrupt newer data.

Ignoring an inaccessible event in the frontend is defense in depth only. The
server must prevent unauthorized group membership and event delivery as
defined in section 17.6.

## 11. Playwright Configuration

The first Playwright implementation must use Chromium only in pull requests.
Firefox may run on a weekly schedule after the Chromium suite is stable.
WebKit is deferred unless Citadel declares Safari support.

Configure:

- `baseURL` from `CITADEL_E2E_BASE_URL`;
- one worker in CI initially;
- full isolation between tests;
- zero retries locally;
- one retry in CI for diagnostic trace collection;
- trace on first retry;
- screenshot on failure;
- video retained on failure;
- HTML and JUnit reports;
- a test timeout appropriate for UI actions;
- longer explicit timeouts only for tagged deployment or backup operations.

A retry must not silently convert a flaky test into an accepted result.
CI must report flaky tests distinctly, and repeated flakes require an owner and
fix.

Use semantic locators. Add `data-testid` only for cases such as Monaco, xterm,
canvas output, dynamically streamed task records, or icon-only state with no
suitable accessible locator.

Never use fixed sleeps. Use Playwright assertions, API polling, visible task
state, or backend state queries with bounded timeouts.

## 12. E2E Environment

`test/Citadel.Tests.E2E/compose.yml` must define a disposable base environment:

- PostgreSQL;
- the Citadel image under test;
- an isolated Docker daemon;
- named internal network;
- ephemeral data volumes;
- health checks.

CI must use a dedicated Docker-in-Docker daemon. Share that daemon's Unix
socket with the Citadel container through a test-owned named volume. Do not
mount the GitHub runner or CI host `/var/run/docker.sock` into Citadel.

The Docker-in-Docker service must:

- use a pinned Docker image;
- run only in the disposable E2E network;
- expose no unauthenticated TCP daemon port to the host;
- use an ephemeral graph-data volume;
- be removed with all volumes during teardown;
- be labeled with the E2E run identifier.

Local development may support the host Docker socket only as an explicit
opt-in, for example `CITADEL_E2E_ALLOW_HOST_DOCKER=true`. The default local
path remains the isolated daemon. Host-socket mode must print a destructive
test warning and use the same run labels and cleanup filters as CI.

Use Compose profiles for optional services:

- `oidc`: Keycloak;
- `secrets`: Vault;
- `s3`: RustFS;
- `git`: Forgejo;
- `edge`: Citadel Edge Agent.

The base PR smoke environment must not start all optional profiles.

Every CI run must use a unique Compose project name. Teardown must execute with
volume and orphan removal even after test failure. Before removing resources,
collect logs and relevant container inspection output.

Container ports exposed to the host should use dynamically assigned or
job-specific ports where practical. Services inside Compose must communicate
through network aliases, not host-only URLs.

## 13. Authentication And Test Personas

### 13.1 Normal Authentication

Ordinary browser tests must authenticate through Citadel's public login API.
Use Playwright's API request context to log in and save browser storage state
under `playwright/.auth`.

The saved state:

- must be generated for each run;
- must be excluded from Git;
- must be treated as a secret-bearing artifact;
- must not be uploaded with test reports;
- must include the refresh cookie needed by the frontend bootstrap flow.

Starting an authenticated browser test from saved cookie state intentionally
exercises `AuthProvider` silent refresh. It must not inject an access token into
local storage or JavaScript state.

### 13.2 Required Personas

Create isolated state for:

- system administrator;
- global read-only user;
- resource-scoped operator;
- user with no access to the target resource.

Create users, roles, teams, and grants through public Citadel APIs wherever
possible. When public APIs cannot create the required precondition, add a
test-only seeding executable that uses Citadel Application and Infrastructure
services directly. It must run as a separate process and must not add an HTTP
route to the production application.

### 13.3 OIDC Authentication

OIDC tests must not reuse local-login storage state. Import a deterministic
Keycloak realm containing:

- one enabled Citadel client;
- valid redirect URIs for the E2E host;
- a known user with verified email;
- a user missing a required claim;
- a disabled or rejected user scenario.

At least one Playwright test must complete the real browser redirect through
Keycloak and return to Citadel. Detailed issuer, signature, audience, nonce,
state, and provisioning failures remain backend integration or acceptance
tests.

## 14. Test Data Isolation

Every test must create its own resource names using a readable prefix plus a
random suffix, for example:

```text
e2e-stack-rollback-7f3a2c
```

Use public APIs for setup and cleanup when they are part of the supported
contract. A UI test should create prerequisites through API requests and use
the browser only for the behavior it is proving.

Docker resources must include labels that identify:

- the E2E run;
- the test;
- the owning Citadel resource.

Cleanup must remove:

- stacks and containers;
- networks and volumes created by the test;
- temporary Git repositories;
- backup snapshots and buckets;
- test users and grants where supported;
- generated authentication state.

Initial Docker-mutating browser tests must run with one worker. Parallelism may
be enabled only after resource names, ports, projects, volumes, and cleanup are
proven isolated.

## 15. Mandatory Frontend Behavior Tests

The first implementation slice must include these tests.

### 15.1 Authentication

- Bootstrap refresh success authenticates and renders the requested route.
- Bootstrap refresh `401` renders login without an infinite request loop.
- Expired access token performs one refresh and retries the operation.
- Failed refresh clears authentication and preserves the intended return URL.
- Logout clears frontend authentication and invokes the logout endpoint.
- `RequireAuth` waits for bootstrap before redirecting.
- `RequireNoAuth` returns an authenticated user to the stored route.

### 15.2 Resource Forms

- Form loads initial data and preserves the API model.
- Required-field validation blocks submission.
- API validation errors map to the correct control.
- Successful create navigates to the expected resource route.
- Successful patch invalidates the list and detail queries.
- Unsaved or server-error state does not falsely report success.
- Form navigation links scroll smoothly to the target section.

Choose representative forms that exercise the shared FormBuilder and resource
configuration infrastructure. Do not duplicate every shared assertion for
every resource.

### 15.3 React Query And SignalR

- A create/update event inserts or updates one list item without duplication.
- A delete event removes the item.
- An event for another resource or scope is ignored.
- Unmounting and remounting a tab removes and restores listeners correctly.
- Reconnect rejoins the required SignalR group.
- A failed mutation does not invalidate unrelated query groups.

### 15.4 Shared User Interfaces

- Sidebar expanded and collapsed navigation remains reachable.
- Collapsed submenus are not clickable through invisible layout.
- Alert count and alert details stay consistent.
- Global search handles loading, results, empty results, authorization, and
  keyboard navigation.
- Resource tabs preserve or reload content correctly after switching.

## 16. Browser Test Portfolio

The initial blocking pull-request browser suite must stay small. It contains:

1. Local login opens the authenticated application, and reload plus browser
   tab refocus preserve the authenticated route.
2. Sidebar navigation reaches a resource list and details route.
3. A representative resource can be created, edited, and deleted.
4. A restricted user cannot see or directly open an inaccessible resource.
5. One focused runtime journey deploys a stack, deploys a second release,
   rolls back to the first release, and verifies the running state.

Journey 5 may run as an independent blocking `core-runtime` job so Docker image
pulls and stack operations do not delay fast browser feedback.

Initially run these browser scenarios nightly rather than blocking every pull
request:

- Logs, Inspect, Terminal, and Logs tab lifecycle against real Docker;
- real SignalR disconnect, reconnect, group rejoin, and visible update;
- visual regression;
- Firefox coverage;
- full external-service browser journeys.

Vitest and backend integration tests still cover tab lifecycle, SignalR cache
behavior, listener cleanup, group authorization, and event filtering on every
pull request.

A nightly runtime scenario may become pull-request blocking only after:

- its resources and cleanup are isolated;
- its failure output is actionable;
- it has no unresolved flake over at least 30 consecutive scheduled runs;
- its measured duration fits the agreed pull-request feedback budget.

## 17. Reliability And Recovery

Long-running operations must be tested at failure boundaries, not only on the
successful path. Most tests in this section belong in backend integration or
.NET acceptance suites. Playwright should cover only user-visible recovery
state for selected scenarios.

### 17.1 Operation Invariants

Every long-running apply, rollback, build, backup, restore, automation, and
agent-routed operation must preserve these invariants where applicable:

- one resource cannot have two conflicting active operations;
- an operation has a durable identity that survives stream disconnection and
  Core restart;
- a stale operation cannot commit state over a newer operation;
- retrying the same accepted request does not create duplicate releases, runs,
  deployments, or child resources;
- a successful state is recorded only after the external side effect succeeds;
- an external side effect that succeeded but lost its response is reconciled
  without being repeated blindly;
- a failed rollback does not falsely mark the target release as running;
- interruption leaves an observable failed, interrupted, or recoverable state;
- startup reconciliation clears or repairs orphaned `Processing` control state;
- cancellation is explicit and is not implied by a browser or progress-stream
  disconnect.

Where an API operation may be retried after a timeout, use a durable operation
or idempotency identifier. A transient in-memory lock alone is not sufficient
to prove retry safety across process restart.

### 17.2 Stack Apply And Rollback Recovery

Required integration and acceptance coverage:

- stack apply fails after some containers are created;
- Core stops after persisting `Processing` but before Docker execution;
- Core stops after Docker succeeds but before database completion;
- Core restarts during apply and during rollback;
- the same apply request is submitted concurrently;
- two users submit conflicting apply and rollback operations;
- retry after client timeout does not create a duplicate release;
- rollback itself fails after partially changing Docker state;
- an older operation completes after a newer release and cannot overwrite it;
- Docker returns partial, stale, or ambiguous container state;
- startup reconciliation repairs or explicitly fails orphaned control state.

Assertions must compare both Citadel state and actual Docker state. A test must
not pass because only the database or only Docker appears correct.

Use dependency-injected barriers and connector test doubles in
`WebApplicationFactory` tests for deterministic failure points. Candidate-image
acceptance may stop the Core container, interrupt the isolated Docker daemon,
or inject a failing compose workload. Do not add public failure-injection
endpoints to Citadel.

### 17.3 Progress And Connection Recovery

Required coverage:

- disconnecting a SignalR or HTTP progress stream does not cancel the operation;
- reconnecting observes the same durable operation rather than creating one;
- duplicate subscribers do not duplicate underlying execution;
- losing the final progress message still leaves queryable terminal state;
- agent disconnect marks routing unavailable without completing the operation
  successfully;
- agent reconnect resumes or safely reports interruption according to the
  operation contract;
- revocation during disconnection prevents the agent from resuming work.

### 17.4 Webhook And Scheduler Idempotency

Required backend integration coverage:

- duplicate webhook delivery does not create duplicate releases, builds, or
  deployments;
- replayed and invalid signatures are rejected;
- out-of-order webhook events cannot move a repository or stack back to an
  older commit;
- a scheduled automation remains discoverable after Core restart;
- a claimed run is not executed twice after worker restart;
- overlapping runs follow the action's configured concurrency policy;
- revoking the run-as actor's permission prevents future execution;
- timezone and daylight-saving transitions are deterministic;
- backup and automation schedules make the same due/not-due decision for the
  same clock value.

Scheduler code must use an injectable `TimeProvider` rather than direct
`DateTime.UtcNow` or `DateTimeOffset.UtcNow` reads before deterministic restart,
timezone, and daylight-saving tests are considered complete.

### 17.5 Upgrade And Migration Safety

Once a stable release exists, every stable candidate must be tested against the
immediately previous stable Citadel version.

The upgrade suite must:

1. start the previous released Core image with its supported PostgreSQL image;
2. create representative users, teams, roles, grants, encrypted credentials,
   platforms, stacks, releases, activities, license state, schedules, backup
   resources, Git resources, and agent bindings;
3. stop the previous Core while preserving PostgreSQL and Citadel data volumes
   plus required encryption configuration;
4. start the candidate image and run normal startup migrations;
5. verify authentication, decryption, permissions, resource history, licensing,
   scheduled work, and agent reconnection;
6. restart the candidate and verify the same state again;
7. verify new candidate operations can use the upgraded resources.

For the first stable release, create a versioned pre-release baseline fixture
and run the same restore-and-upgrade process so the mechanism exists before it
is needed. Prefer a deterministic logical database fixture or seeding program;
do not commit a PostgreSQL data-directory archive tied to one host layout.

Where the migration mechanism supports transactional failure, inject or use a
known failing migration and prove the application does not become healthy with
partially accepted schema state. Always preserve logs and enough database state
to diagnose migration failures.

### 17.6 Server-Side Authorization Boundaries

Frontend hiding and event filtering are not authorization controls.

Create a compact backend permission matrix covering:

- administrator bypass;
- direct global role permission;
- permission inherited through a team role;
- resource-specific actor and team grants;
- no relevant grant;
- disabled actor;
- required `Read`, `Write`, and `Execute` levels;
- representative specific permissions such as `Logs`, `Terminal`, `Apply`,
  `Releases`, `Restore`, and `Browse`.

For representative protected resource types, assert the matrix against:

- list filtering;
- detail lookup;
- mutation and direct API calls;
- resource lookup endpoints;
- global search names and secondary metadata;
- SignalR group join;
- SignalR event recipient selection;
- log, terminal, task, and activity streams.

Unauthorized users must not join protected groups and must not receive
protected events. The test must inspect both hub invocation failure and actual
recipient connections. Keep only one or two representative browser journeys;
the complete matrix belongs in backend integration tests.

SignalR group authorization must fail closed. Shared resource-list groups
require global `Read` permission for that resource type; a resource-specific
grant authorizes only the matching detail group. Activity groups include both
the activity resource type and resource ID so the server can apply the correct
permission. Alert clients join only the public `alert-events` alias, which the
server maps to a user-specific group after database-side recipient selection;
clients must never be allowed to join another user's concrete alert group.

### 17.7 Licensing And Capability Boundaries

Add backend integration coverage for:

- no installed license uses the Community baseline with no entity-count quotas;
- Community can create core resources without licensing count boundaries;
- capability-protected operations fail with a stable entitlement error;
- invalid signature, malformed payload, wrong-instance, and expired licenses
  are rejected; a valid future-dated license may be installed as
  `NotYetValid`, but only Community capabilities remain effective until
  `notBefore`;
- existing resources remain usable according to the documented expiry policy;
- downgrade does not destructively delete or rewrite paid configuration;
- scheduled workers, webhooks, and direct API calls cannot bypass a missing
  capability;
- Community accepts and authenticates repository webhooks, synchronizes source,
  and records pending updates without starting a paid mutating operation;
- external build-pool execution requires its capability while build-pool
  configuration and manual builds on existing managed platforms remain
  available;
- webhook-triggered external-pool builds require both the trigger and
  execution-target capabilities;
- missing or mismatched replacement IDs and future-dated replacements of an
  active license fail with stable conflict responses;
- schema-1 Business licenses map to the documented Team compatibility bundle;
- UI capabilities and backend enforcement derive from the same effective
  license state;
- authenticated users can read effective entitlements without receiving
  administrative license metadata;
- a license transition invalidates relevant cached capabilities.

Add one browser journey proving a representative locked feature is explained
and cannot be bypassed by direct navigation. The backend matrix remains
authoritative.

### 17.8 Citadel Control-Plane Disaster Recovery

Workload backup and Citadel's own control-plane recovery are separate
guarantees.

Create a scheduled and release-gated disaster-recovery test that:

1. populates a Citadel installation with representative identity, permission,
   encrypted secret, platform, stack release, Git, backup, activity, schedule,
   license, and agent-binding state;
2. performs the documented PostgreSQL and persisted-data backup procedure;
3. preserves required encryption keys and non-database recovery inputs through
   the documented secure mechanism;
4. destroys the original Core, database, and persistent volumes;
5. restores into a clean environment;
6. starts the same candidate image;
7. verifies authentication, decryption, authorization, resources, histories,
   schedules, licensing, and agent reconnection;
8. performs one new operation after restore.

The test must fail if database data is restored without required encryption
material. Recovery documentation and automation must identify all required
inputs; copying only the PostgreSQL database is not considered a complete
Citadel backup.

## 18. Compatibility Acceptance Suites

### 18.1 Keycloak OIDC

Use a pinned Keycloak image and realm import.

Required acceptance coverage:

- discovery succeeds against real Keycloak metadata;
- full authorization-code and PKCE browser login succeeds;
- Citadel creates its own refresh session;
- browser reload after OIDC login remains authenticated;
- provisioning and linking follow provider configuration;
- missing required claim is rejected;
- disabled provider cannot start a new login;
- logout clears the Citadel session.

### 18.2 Vault-Compatible KV v2

Use a pinned Vault image in disposable dev mode with a fixed test-only root
token and the default KV v2 mount.

Required acceptance coverage:

- provider connection test succeeds;
- a secret value resolves from KV v2;
- a stack or deployment receives the resolved secret;
- the secret is not exposed in UI output, activity details, or task logs;
- invalid token returns an actionable error;
- missing path or key returns an actionable error;
- provider token replacement and preservation behavior works.

### 18.3 RustFS S3-Compatible Backups

Use a pinned RustFS image in single-node disposable mode.

Required acceptance coverage:

- create or prepare an isolated bucket;
- validate an S3-compatible backup repository;
- back up known source bytes;
- remove or alter the source;
- restore the selected snapshot;
- compare byte count and cryptographic checksum;
- verify failed credentials and missing bucket behavior;
- verify retention or deletion affects only the test prefix.

The integrity assertion belongs in .NET acceptance tests. One selected browser
test must prove that a user can configure the repository and start or inspect a
run.

### 18.4 Forgejo Git And Webhooks

Use a pinned Forgejo image with deterministic initial configuration.

Required acceptance coverage:

- create a user, token, repository, branch, and initial commit;
- configure a Forgejo webhook to Citadel;
- discover or configure the repository in Citadel;
- deploy a Git-backed stack from the initial commit;
- push a second commit;
- receive and authenticate the real webhook;
- create or update the expected Citadel release or drift state;
- deploy the new release;
- reject an invalid webhook signature;
- ignore changes outside configured watch paths.

Direct webhook payload tests remain in backend unit/integration suites. This
suite proves actual Forgejo compatibility and delivery.

### 18.5 Edge Agent

Use the Citadel Agent image version intended to be compatible with the Core
commit under test.

Required acceptance coverage:

- create an enrollment;
- start the agent with the enrollment data;
- observe connected status;
- execute one supported platform operation through the agent;
- stream progress or logs to the UI;
- restart the agent and reconnect;
- revoke the binding and reject reconnection;
- avoid delivering another resource's commands to the agent.

The test workflow must accept Core and Agent image references as inputs so a
release can validate their compatibility before publication.

## 19. Selected Visual And Accessibility Coverage

Do not snapshot every component.

Add stable Playwright screenshots for:

- authenticated application shell with header and sidebar;
- collapsed sidebar;
- representative resource table;
- representative FormBuilder form;
- global search popup;
- task or log sheet.

Capture desktop and one supported mobile viewport. Disable animations and use
deterministic data and time for screenshot tests.

Run focused axe checks on:

- login;
- application shell;
- resource table;
- resource form;
- dialog or sheet with validation errors.

Accessibility violations fail the test unless a documented, narrowly scoped
exception references an issue.

## 20. CI Workflows

### 20.1 Pull Request CI

Create or refactor workflows so these jobs can run independently:

#### `frontend`

```text
npm ci
npm run prettier:verify
npm run lint
npm run build:development
npm run test:run
```

#### `backend-unit`

Restore, build, and run `Citadel.Tests.Unit`.

#### `backend-integration`

Restore, build, and run `Citadel.Tests.Integration`.

#### `e2e-smoke`

Build the local Citadel image, start the base E2E environment, install
Playwright Chromium, run the smoke suite, collect artifacts, and tear down.

#### `core-runtime`

Run the single stack deploy, second release, and rollback journey against the
isolated Docker daemon.

All five jobs block merge. The first implementation does not make the extended
logs/terminal lifecycle, browser SignalR reconnect, visual, Firefox, or
external-service browser suites pull-request blocking.

Track these initial feedback objectives:

- frontend behavior checks: under 3 minutes;
- core browser smoke: under 10 minutes;
- all pull request checks: under 15 minutes when jobs run in parallel.

These are measured engineering aspirations, not first-slice acceptance
criteria and not reasons to remove required coverage or add unsafe
parallelism. Record actual duration and flake rate before changing job
placement.

### 20.2 Nightly Acceptance

Run a scheduled and manually dispatchable matrix:

| Matrix job | Profiles or services |
| --- | --- |
| `git-webhook` | Forgejo and Docker |
| `edge-agent` | Core, Agent, Docker |
| `backups-s3` | RustFS |
| `oidc` | Keycloak |
| `secrets` | Vault |
| `reliability` | Core restart, Docker interruption, and operation recovery |
| `upgrade` | previous Core release, candidate Core, PostgreSQL |
| `control-plane-recovery` | backup, destruction, and clean restore |
| `scheduler-webhook` | deterministic clock, restart, duplicate, and replay |
| `licensing` | capability boundaries, transitions, downgrade, and legacy-license compatibility |
| `authorization` | API, lookup, search, SignalR group, and recipient matrix |
| `browser-extended` | selected visual, accessibility, and Firefox tests |

Matrix jobs must be independent so one service failure does not hide results
from other suites.

### 20.3 Release Gate

Stable release publication must:

1. build an immutable candidate image or manifest for the release commit;
2. run core smoke, recovery, upgrade, control-plane restore, authorization,
   licensing, and required compatibility acceptance against that candidate;
3. record the tested image digest;
4. promote or copy the tested digest to stable GHCR and Docker Hub tags;
5. sign the published digest;
6. stop publication if any required acceptance suite fails.

Do not test one image and rebuild unrelated release bits afterward. If registry
promotion across GHCR and Docker Hub requires a copy, verify the destination
manifest contents and record both digests.

After the first stable release, upgrade from the immediately previous stable
version is mandatory for every stable release. Control-plane disaster recovery
must run on a schedule and before stable publication whenever persistence,
encryption, identity, licensing, migrations, agent binding, or backup packaging
changes.

## 21. OpenAPI And Generated Client Gate

Add a CI check that detects drift among:

- the Web API OpenAPI output;
- checked-in schema snapshots;
- generated frontend API types;
- generated frontend resource metadata.

The check must regenerate deterministic output and fail when a Git diff
remains. It must not silently modify CI artifacts and continue.

Generated code is compile-tested but excluded from behavior coverage metrics.

## 22. Failure Diagnostics

Every failed browser job must upload:

- Playwright HTML and JUnit reports;
- trace for the failed test or retry;
- failure screenshot;
- retained failure video;
- browser console errors;
- failed API responses and request URLs with secrets redacted;
- Citadel container logs;
- PostgreSQL and optional service logs when relevant;
- `docker compose ps`;
- inspected state for test-owned containers when relevant.

Acceptance tests must include service logs in the failure message or artifact.
Secrets, access tokens, refresh cookies, OIDC codes, Vault tokens, registry
credentials, and backup passwords must be redacted before output or upload.

## 23. Flake Policy

A flaky test is a defect in either the product, test, or environment.

- Do not add fixed delays to hide races.
- Do not increase global timeouts to hide one slow operation.
- Do not use retries as the only fix.
- Quarantining a test requires a tracked issue, owner, reason, and removal date.
- A quarantined test must continue running in a non-blocking nightly job.
- A test that flakes repeatedly must not remain in the blocking suite without
  corrective work.

Use deterministic readiness checks, visible state transitions, polling with a
bounded timeout, and isolated resources.

## 24. Coverage Policy

Collect V8 coverage for frontend behavior tests and publish the report.

The first slice must establish a baseline without a global percentage gate.
After the initial portfolio is stable:

- enforce no meaningful coverage regression in critical modules;
- set focused thresholds for authentication, shared form logic, query/cache
  reducers, permission mapping, and SignalR adapters;
- exclude generated API code, type-only files, icons, and shadcn primitives;
- do not add low-value assertions solely to increase coverage.

Coverage is a navigation aid. Mandatory workflows and regression tests remain
the actual quality gate.

## 25. Security Requirements

- Never commit Playwright authentication state.
- Never upload authentication state as a CI artifact.
- Use only disposable test credentials.
- Bind externally exposed test service ports to loopback where possible.
- Keep test services on an isolated Docker network.
- Do not use production secrets or shared cloud accounts.
- Redact HTTP headers, cookies, tokens, secret values, and credentials.
- Verify that secret-related acceptance tests assert non-disclosure.
- Disable public registration and unnecessary service capabilities in Forgejo
  and Keycloak fixtures.
- Use Vault dev mode and insecure HTTP only inside the disposable test
  environment.
- Clean test volumes after every run.

## 26. Implementation Slices

### Slice 1: Frontend Foundation

1. Add Vitest, Testing Library, user-event, jest-dom, jsdom, and MSW.
2. Add configuration, setup, strict MSW server, response factories, and
   `renderCitadel`.
3. Add frontend scripts and frontend CI job.
4. Add authentication bootstrap and route-guard tests.
5. Add representative form tests.
6. Add Query Client and SignalR adapter tests.
7. Publish coverage without enforcing a global threshold.

Slice 1 is complete when at least the mandatory authentication tests and one
representative set from each frontend category in section 15 pass in CI.

### Slice 2: Core Browser Smoke

1. Create the independent Playwright project.
2. Create the base Compose environment with isolated Docker-in-Docker.
3. Add API-based setup helpers and authenticated storage-state generation.
4. Add journeys 1 through 4 from section 16.
5. Add traces, reports, screenshots, logs, and reliable teardown.
6. Add the blocking `e2e-smoke` PR job.

### Slice 3: Stack Deploy And Rollback

1. Add the focused deploy, second release, and rollback journey.
2. Verify Citadel release state and actual Docker state.
3. Add the independent blocking `core-runtime` job.
4. Add Logs/Inspect/Terminal lifecycle and real SignalR reconnect scenarios to
   nightly execution initially.
5. Measure duration and flake rate before promoting additional runtime tests to
   pull-request blocking.

### Slice 4: Reliability And Release Safety

1. Add deterministic operation barriers and recovery integration tests.
2. Add idempotency, concurrency, stale-operation, and startup reconciliation
   tests.
3. Add the backend authorization and SignalR recipient matrix.
4. Add licensing capability, downgrade, and legacy-license tests.
5. Refactor schedulers to use `TimeProvider` and add restart, timezone, and
   duplicate-run coverage.
6. Add the pre-release upgrade baseline, previous-version upgrade harness, and
   migration verification.
7. Add Citadel control-plane backup, destruction, and restore automation.
8. Add Agent disconnect, reconnect, and revocation recovery coverage.

Current upgrade baseline implementation:

- `test/Citadel.Tests.Acceptance` is the independent xUnit product-acceptance
  project.
- `Upgrades/PreReleaseUpgradeTests.cs` restores the frozen pre-release `1.0.0`
  schema and representative seed, starts the candidate as a real process,
  verifies authentication and preserved state, performs a new operation,
  restarts the candidate with the same database and data directory, and
  verifies another operation.
- The normalized SHA-256 of `script0001.sql` is pinned so the historical
  baseline cannot be edited silently.
- A failing transactional migration is verified to leave neither partial
  schema nor a journal entry.
- `.github/workflows/docker-publish.yml` runs this baseline as the blocking
  `upgrade-baseline` job and includes it in stable publication dependencies.

After the first stable image is published, keep this pre-release baseline as a
migration regression fixture and add the previous-stable-image path described
in section 17.5. The image-based path must seed state through the previous
release, preserve its PostgreSQL and Citadel data volumes, and start the tested
candidate image rather than rebuilding a different candidate.

Current control-plane recovery implementation:

- `Recovery/ControlPlaneRecoveryTests.cs` creates a real custom-format
  PostgreSQL dump and a checksum-validated package containing only the
  file-backed JWT, secret-encryption, Core-to-Agent, and data-protection key
  material.
- The suite destroys the source database and data directory, restores into a
  new empty database and data directory, starts the same built candidate
  artifact, verifies representative identity, permissions, encrypted secret,
  platform, stack release, Git, backup history, activity, schedule, license,
  and agent-binding state, and performs a new operation.
- A package missing `secret-encryption-key` is rejected before the target
  database is modified.
- `.github/workflows/docker-publish.yml` runs the suite on schedule, on demand,
  and before stable publication through the `control-plane-recovery` job.
- `docs/user/control-plane-recovery.md` documents the manual PostgreSQL and
  recovery-asset procedure until the product `Citadel.Recovery` command and
  packaged Citadel System backup slices are implemented.

This acceptance harness validates the recovery contract but does not make the
current Citadel System restic backup a complete control-plane backup. Product
integration remains owned by the system-backup and offline-recovery slices in
`docs/specs/backup-and-restore.md`.

Current Edge Agent recovery implementation:

- `Recovery/EdgeAgentRecoveryTests.cs` starts the candidate Core as a real
  process and connects a protocol-level Edge Agent client over the public
  HTTP/2 stream.
- The suite enrolls the agent, sends a heartbeat, and completes a routed
  platform operation. It then proves that disconnect reports the platform
  offline and rejects routed work with `503`, authenticated reconnect restores
  operation, and reconnect after a Core process restart restores operation.
- The suite revokes the binding while disconnected and verifies that the same
  persisted identity cannot reconnect or receive more commands.
- Session replacement uses atomic key/value removal so cleanup from an older
  stream cannot remove or mark a newer session offline.
- `.github/workflows/docker-publish.yml` runs the suite on schedule, on demand,
  and before stable publication through the `edge-agent-recovery` job.

This reliability slice intentionally exercises the Core protocol without
checking out or launching the separate Agent repository. The real-image suite
below owns image compatibility and Docker-backed behavior.

### Slice 5: Compatibility Acceptance

Implement in this order:

1. Forgejo Git stack and webhook;
2. Edge Agent;
3. RustFS backup and restore;
4. Keycloak OIDC;
5. Vault KV v2.

Move Keycloak earlier only when OIDC is declared a requirement for the initial
public release.

Each service slice adds .NET acceptance coverage first and only the minimum
browser coverage needed to prove the user-facing wiring.

Current Forgejo compatibility implementation:

- `Compatibility/ForgejoGitWebhookTests.cs` starts the candidate Core and
  `codeberg.org/forgejo/forgejo:16.0.1-rootless` as disposable processes.
- The fixture disables registration, creates an administrator, access token,
  private repository, `main` branch, and real Git commits.
- The suite configures a private Git repository and Git-backed stack through
  Citadel's public API, then verifies the initial deployment and its resolved
  commit.
- A real Forgejo push webhook signed with `X-Gitea-Signature` updates the stack;
  applying it creates the expected second release and resolved commit.
- The suite rejects an invalid signature and proves that a commit outside the
  configured stack watch paths produces a no-op without advancing the deployed
  release or repository cache.
- Application-owned Git caches and stack snapshots resolve from the process
  working directory, preserving `/app/data` in containers while allowing the
  candidate process to use its isolated acceptance data directory.
- `.github/workflows/docker-publish.yml` runs the `git-webhook` job on schedule,
  on demand, and before stable publication.

Current Edge Agent compatibility implementation:

- `Compatibility/EdgeAgentCompatibilityTests.cs` starts supplied Core and
  Agent images on an isolated Testcontainers network with a dedicated
  Docker-in-Docker daemon.
- The suite creates a real enrollment, checks the connected protocol and Agent
  version, routes Docker operations, and proves that a different platform
  cannot deliver commands to the connected Agent.
- It deploys a BusyBox stack through the Agent, verifies streamed progress and
  completion, and inspects the running container through the live Agent route.
- The Agent identity directory is persisted across a container restart. The
  suite verifies authenticated reconnect, then revokes the binding and proves
  that the persisted identity is rejected and cannot receive commands.
- `.github/workflows/docker-publish.yml` runs the
  `edge-agent-compatibility` job on schedule, on demand, and before stable
  publication. Manual runs accept exact Core and Agent image references, an
  Agent Git ref, and an optional expected Agent version. Stable tags build the
  Core tag under test and default to the matching Agent tag.

Current RustFS compatibility implementation:

- `Compatibility/RustFsBackupCompatibilityTests.cs` runs the supplied Core
  image against pinned RustFS and a dedicated Docker-in-Docker platform.
- The candidate Core image is imported into the platform daemon and used as
  the real backup helper, so repository validation, backup, retention, prune,
  and restore execute through the production container path.
- The suite creates an isolated bucket and repository prefix, initializes and
  validates the repository, and verifies actionable failure state for invalid
  credentials and a missing bucket.
- It backs up deterministic Docker volume bytes, alters the source, restores
  the selected snapshot to a new volume, and compares both byte count and
  SHA-256.
- A second snapshot exercises `keep-last` retention, an explicit prune
  completes, and an object under a separate prefix remains present.
- `.github/workflows/docker-publish.yml` runs the `backups-s3` job on schedule,
  on demand, and before stable publication. The job reuses the exact Core image
  artifact that passed the browser smoke suite.
- Candidate-image tests skip during an ordinary aggregate local run when their
  image variables are absent. Dedicated compatibility jobs set
  `CITADEL_ACCEPTANCE_REQUIRE_CANDIDATE_IMAGES=true`, which turns missing image
  configuration into a test failure so a skipped suite cannot satisfy a
  release gate.
- `test/Citadel.Tests.Acceptance/run-image-compatibility.ps1` builds or validates
  the local Core and Agent candidate images, requires both suites to execute,
  and restores the caller's process environment after the run.

The selected browser test for configuring a repository and starting or
inspecting a run remains a separate user-facing slice; integrity and storage
isolation stay owned by this .NET acceptance suite.

Current Keycloak compatibility implementation:

- `fixtures/keycloak/citadel-e2e-realm.json` defines a deterministic realm,
  confidential PKCE client, verified auto-link user, missing-claim user, and
  disabled user.
- `Compatibility/KeycloakOidcCompatibilityTests.cs` starts Keycloak by
  immutable image digest and a real candidate Core process. It verifies
  discovery, authorization-code login startup with PKCE, client-secret
  isolation, and disabled-provider rejection.
- `tests/compatibility/oidc.spec.ts` performs the selected real-browser journey
  without local-login storage state. It verifies email auto-linking, the normal
  Citadel refresh cookie, authenticated reload, required-claim rejection, and
  logout followed by an unauthenticated reload.
- `.github/workflows/docker-publish.yml` runs the `oidc` job on schedule, on
  demand, and before stable publication. The browser journey uses the exact
  candidate image exported by `e2e-smoke`.
- The Compose Keycloak profile and the .NET fixture share one realm import and
  one pinned image digest so protocol and browser scenarios cannot drift.

### Slice 6: Release Gate And Extended Quality

1. Add nightly matrix execution.
2. Add selected visual regression tests.
3. Add focused accessibility tests.
4. Add Firefox nightly coverage.
5. Promote only acceptance-tested release digests.
6. Add focused coverage thresholds after measuring the baseline.

## 27. Acceptance Criteria

### Frontend Foundation

- `npm test` runs in watch mode locally.
- `npm run test:run` runs deterministically in CI.
- Unhandled API requests fail frontend tests.
- Tests use a fresh Query Client and isolated DOM state.
- Authentication bootstrap, refresh failure, route guards, representative
  forms, query updates, and SignalR lifecycle have automated coverage.
- Generated API code and shadcn primitives are not tested as application logic.

### Browser Smoke

- A developer can start the E2E environment and run smoke tests with documented
  commands.
- CI runs the smoke suite in Chromium on every pull request.
- Authentication reload and tab-refocus regressions are covered.
- Authorization is tested with at least one restricted persona.
- Stack deployment, a second release, and rollback are covered in one focused
  blocking runtime journey.
- Logs tab lifecycle and SignalR cache behavior run in fast tests on every pull
  request, with real-browser reconnect scenarios initially running nightly.
- The CI environment uses an isolated Docker daemon and does not mount the
  runner's Docker socket.
- Failed tests produce enough artifacts to diagnose browser, API, and service
  state.
- Teardown removes containers, volumes, networks, and authentication state.

### Reliability And Recovery

- Apply and rollback interruption points have deterministic integration tests.
- Duplicate, concurrent, retried, and stale operations cannot create duplicate
  state or overwrite a newer result.
- Core restart repairs or explicitly fails orphaned processing state.
- Progress-stream disconnection does not cancel or duplicate execution.
- Agent disconnect, reconnect, and revocation preserve operation and
  authorization invariants.
- Webhook and scheduler duplicate/restart behavior is idempotent.

### Authorization And Licensing

- Unauthorized users cannot join protected SignalR groups or receive protected
  events.
- API, lookup, global search, logs, terminal, and stream boundaries use the
  same effective permission rules.
- Global roles, team roles, resource grants, specific permissions, disabled
  actors, and administrator bypass are covered by a compact matrix.
- Community baseline, capability denial, invalid-license, replacement conflict,
  expiry, downgrade, automated-trigger enforcement, external-build execution,
  and legacy Business compatibility are covered.

### Upgrade And Control-Plane Recovery

- The candidate upgrades a representative previous-version installation.
- Upgraded encrypted data, identity, permissions, releases, schedules,
  licensing, and agent bindings remain usable after a second restart.
- A clean environment can restore the documented Citadel control-plane backup.
- Missing encryption material causes an explicit recovery failure.
- One new operation succeeds after upgrade and after disaster recovery.

### Compatibility

- Keycloak OIDC login completes in a real browser.
- Vault KV v2 resolves a secret without disclosing it.
- RustFS backup and restore reproduce the original byte checksum.
- Forgejo delivers a signed webhook that updates a Git-backed stack workflow.
- Edge Agent enrolls, executes an operation, reconnects, and respects
  revocation.
- All compatibility services use pinned, disposable containers.

### CI And Release

- Frontend, backend unit, backend integration, browser smoke, and the focused
  core runtime job block pull-request merges.
- Product acceptance suites run nightly and on demand.
- Stable releases cannot publish when required recovery, upgrade,
  control-plane restore, authorization, licensing, or compatibility acceptance
  fails.
- The tested candidate digest is recorded and promoted to stable tags.
- Test reports never expose credentials, tokens, cookies, or resolved secret
  values.

## 28. Final Design Rules

- Test behavior, not implementation details.
- Use real Citadel APIs in E2E setup; use MSW only in frontend behavior tests.
- Keep browser journeys short and independent.
- Validate protocol compatibility and data integrity outside the browser.
- Use one real browser journey per important external integration, not a second
  backend test suite written as clicks.
- Treat restart, retry, duplicate delivery, stale completion, and partial
  external success as normal test cases for long-running operations.
- Treat authentication, authorization, stack deployment, rollback, backups,
  upgrades, control-plane recovery, licensing, secrets, webhooks, and Edge
  Agent routing as release-critical behavior.
- Add a regression test whenever an escaped bug can be automated.
- Prefer deterministic isolation and useful diagnostics over maximum
  parallelism.
