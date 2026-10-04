# CI and release configuration

Normal pushes and pull requests validate the current code. Publishing is reserved
for matching `vMAJOR.MINOR.PATCH` tags on `main`.

## Checks on a pull request

| Workflow | Checks |
| --- | --- |
| CI | Rust source ownership and isolated build context; formatting; generated Docker API, database schema, OpenAPI and frontend client; workspace checks, Clippy and tests; database regressions; frontend types and unit tests |
| Documentation | Content and links, site configuration regressions, lint and types, static product export, public schema validation, independent ReDoc export, and smoke checks |
| Agent images | Native amd64 and arm64 validation, packaged runtime smoke checks, Direct/Edge/live Docker tests, complete Core/Agent acceptance, and released Agent compatibility when a baseline is configured |

Rust dependency caches are separated between Core and Agent architectures. Agent
jobs reclaim unused SDK space on disposable GitHub-hosted runners before building
both production images. The cleanup is skipped on self-hosted runners. Database
fixtures must use unique Docker service IDs and bootstrap token hashes because
regression suites share one database and the production schema enforces uniqueness.

Agent compatibility executables are compiled in `Dockerfile.agent`'s pinned `rust-source`
stage, then run inside the tested Agent image. Host-built binaries can require a
newer glibc than that runtime and must not be substituted for the pinned build.
The tests use disposable Docker services rather than the host engine as their target.

Swarm acceptance publishes each candidate as a one-platform image index in its
disposable registry, matching the release format. Docker's distribution endpoint
can omit platform metadata for a single OCI manifest. The index contains only the
candidate's actual architecture; the amd64 and arm64 jobs each test their native image.

## Browser test coverage

The Playwright suites under `test/e2e` are currently manual; these workflows do
not run them or schedule the nightly suite. For changes to browser behavior,
follow [the E2E guide](../test/e2e/README.md) and run smoke, core-runtime, and
accessibility checks against its disposable production-image fixture. These
checks complement the existing Rust, frontend unit, and Agent acceptance jobs.

## Publication settings

Configure these under the repository's **Settings → Secrets and variables → Actions**:

| Name | Kind | Used for |
| --- | --- | --- |
| `DOCS_SITE_URL` | Variable | Actual product-documentation URL, including a project Pages path if applicable |
| `API_DOCS_URL` | Variable | Actual address where the independent API Preview will be hosted |
| `DOCKERHUB_NAMESPACE` | Variable | Namespace receiving the Core and Agent images |
| `DOCKERHUB_USERNAME` | Variable | Account used to publish to Docker Hub |
| `DOCKERHUB_TOKEN` | Secret | Docker Hub publishing token |
| `CITADEL_RUST_AGENT_RELEASE_ENABLED` | Variable | Set to `true` only when Agent publication is ready |
| `CITADEL_AGENT_ROLLBACK_IMAGE` | Variable | Previous compatible Agent image pinned as `image@sha256:…`; required when Agent publication is enabled |
| `ACCESS_TOKEN` | Optional secret | Checkout token override; otherwise workflows use `github.token` |

GHCR uses `github.token` and the workflow's `packages: write` permission. Do not
put publishing credentials in documentation or source files. Repository variables
are not needed for ordinary documentation validation: it uses localhost fallbacks.
Those fallbacks are test addresses and do not configure public hosting.

For product docs, select **GitHub Actions** as the Pages build source and allow
the `github-pages` environment to deploy release tags. If you use a custom domain,
configure it in Pages and use its real URL. The separate API Preview is uploaded
as `citadel-api-docs-TAG`; the current workflow does not publish it to a second site.
Choose its hosting target before advertising that address.

Before enabling the first Rust Agent release, establish and verify an appropriate
rollback baseline. The workflow deliberately rejects an enabled publication with
no digest-pinned baseline; ordinary CI still runs the current-image acceptance suite.

## Run documentation checks locally

From the repository root, with Node.js 24:

```bash
npm ci --prefix docs
export NEXT_PUBLIC_DOCS_URL=http://localhost:3000
export NEXT_PUBLIC_API_DOCS_URL=http://localhost:3001
npm run test:config --prefix docs
npm run validate --prefix docs
npm run lint --prefix docs
npm run types:check --prefix docs
npm run build --prefix docs
npm run test:static --prefix docs
npm run build:api --prefix docs
npm run test:api --prefix docs
```

Run type generation and the Next.js build sequentially because both write to
`docs/.next`. To check a project Pages deployment, rebuild with
`NEXT_PUBLIC_DOCS_URL=http://localhost:3000/Citadel` and repeat the static smoke check.

For Rust, use the pinned toolchain and prerequisites in the workflows. Keep all
formatting, lint, generated-artifact, database, and Agent checks enabled. Regenerate
API artifacts with `cargo run --locked -p xtask -- openapi`; never hand-edit them.

## Review a release

1. Confirm the release commit is on `main` and its tag matches `version.json`.
2. Verify Docker Hub settings, real documentation URLs, Pages configuration, and Agent baseline before tagging.
3. Wait for all required jobs on both Agent architectures to pass.
4. Check image digests and signatures, open the deployed product docs, and deploy the independent API artifact to its chosen host.
5. Confirm installation commands use a complete published image address and test backup/restore before upgrading a persistent installation.
