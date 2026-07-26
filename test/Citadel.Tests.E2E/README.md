# Citadel E2E Tests

The suites run against the production Citadel image, PostgreSQL, and an isolated
Docker-in-Docker daemon. They do not mount the host Docker socket into Citadel.

## Prerequisites

- Docker Desktop or another Docker Engine with Compose support
- Node.js 24 or later

## Run Locally

From the repository root, build the candidate image and install the test
dependencies:

```powershell
docker build -t citadel-e2e:local -f src/Citadel.WebApi/Dockerfile .
Set-Location test/Citadel.Tests.E2E
npm ci
npm run install:chromium
```

Start the isolated environment and run the blocking smoke suite:

```powershell
$env:COMPOSE_PROJECT_NAME = "citadel-e2e-local"
npm run env:up
npm run test:smoke
```

Citadel is available at `http://127.0.0.1:18000` while the environment is
running.

## Test Suites

```powershell
npm run test:smoke            # Fast authentication, authorization, licensing, navigation, and tag checks
npm run test:core-runtime     # Deploy, second release, UI rollback, and Docker-state verification
npm run test:nightly-runtime  # Logs, Inspect, Terminal, and real SignalR reconnect coverage
npm run test:visual           # Chromium desktop and mobile visual regression coverage
npm run test:accessibility    # Focused WCAG A/AA checks for login and representative application surfaces
npm run test:oidc             # Real Keycloak redirect, refresh session, claim policy, and logout
npm run test:vault            # Vault provider connection, save, and stored-token preservation
npm test                      # Run every Playwright test
```

The core runtime suite is a blocking CI check. The runtime lifecycle suite is
initially scheduled nightly because it intentionally exercises a real network
interruption. Both runtime suites create disposable resources whose names start
with `e2e-runtime-` and remove them after each test.

The visual suite covers the authenticated shell and resource table, collapsed
desktop sidebar, Stack FormBuilder, global search results, and a completed task
sheet. Baselines live beside the visual test and are reviewed like code. To
accept an intentional visual change:

```powershell
npm run test:visual:update
```

Run the visual suite with the same Playwright version and operating system used
by CI before committing updated baselines. CI uses Ubuntu and Chromium.

The accessibility suite checks login validation, the authenticated shell,
resource tables, FormBuilder validation, and a resource dialog. It runs in the
blocking E2E job and does not suppress Axe rules globally. Any exception must
be narrowly scoped in the test and reference a tracked issue.

The OIDC suite requires the optional Keycloak profile. It uses a deterministic
realm and empty browser storage rather than the local-login state:

```powershell
npm run env:down
npm run env:oidc:up
npm run test:oidc
npm run env:oidc:down
```

The Playwright Chromium project maps the Keycloak issuer hostname to the
profile's published port and uses `http://127.0.0.1:18000` for Citadel. Run
`npm test` against `env:oidc:up` when executing every Playwright suite together.

The Vault suite requires its optional profile:

```powershell
npm run env:down
npm run env:vault:up
npm run test:vault
npm run env:vault:down
```

The profile uses a fixed test-only root token and is suitable only for the
disposable E2E environment. The browser configures the provider through
Citadel; it does not connect to Vault directly.

Other Playwright modes:

```powershell
npx playwright test --headed    # Run with a visible browser
npx playwright test --ui        # Open Playwright's interactive UI
npx playwright show-report      # Open the most recent HTML report
```

When testing is complete, remove the disposable containers and volumes:

```powershell
npm run env:down
```

Use a unique `COMPOSE_PROJECT_NAME` when running multiple environments. The
generated `.auth` directory contains cookies and must never be committed or
uploaded as an artifact.

If port `18000` is already in use, set both the published port and Playwright
base URL before starting the environment:

```powershell
$env:CITADEL_E2E_PORT = "18001"
$env:CITADEL_E2E_BASE_URL = "http://127.0.0.1:18001"
```
