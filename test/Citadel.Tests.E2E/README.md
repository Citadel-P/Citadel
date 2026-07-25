# Citadel E2E Tests

The smoke suite runs against the production Citadel image, PostgreSQL, and an
isolated Docker-in-Docker daemon. It does not mount the host Docker socket into
Citadel.

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

Other available test modes:

```powershell
npm test                         # Run every Playwright test
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
