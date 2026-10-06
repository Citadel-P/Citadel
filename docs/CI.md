# Continuous integration

Pull requests and branch builds validate code, generated contracts, documentation
and packaged Core/Agent behavior. Contributors can run these checks without
publishing credentials or access to the project's hosting environments.

## Checks on a pull request

| Workflow | Checks |
| --- | --- |
| CI | Pinned native NBGV/version and promotion policy fixtures, digest-transfer testing between two local registries; Rust source ownership and isolated build context; formatting; generated Docker API, database schema, OpenAPI and frontend client; workspace checks, Clippy and tests; database regressions; frontend types and unit tests |
| Documentation | Content and links, site configuration regressions, lint and types, static product export, public schema validation, independent ReDoc export, and smoke checks |
| Reusable Core/Agent candidates | Native amd64 and arm64 validation, both packaged runtime checks, Direct/Edge/live Docker tests and complete Core/Agent acceptance; mixed-version fixtures can also exercise compatibility with released images |

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
