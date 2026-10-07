# Continuous integration

Pull requests and branch builds validate code, generated contracts, documentation
and packaged Core/Agent behavior. Contributors can run these checks without
publishing credentials or access to the project's hosting environments.

## Checks on a pull request

| Workflow | Checks |
| --- | --- |
| CI | Pinned native NBGV/version and promotion policy fixtures, digest-transfer testing between two local registries; Rust source ownership and isolated build context; formatting; generated Docker API, database schema, OpenAPI and frontend client; workspace Clippy and tests; database regressions; frontend types and unit tests |
| Documentation | Content and links, site configuration regressions, lint and types, static product export, public schema validation, independent ReDoc export, and smoke checks |
| Reusable Core/Agent candidates | Native amd64 and arm64 validation, both packaged runtime checks, Direct/Edge/live Docker tests and complete Core/Agent acceptance; mixed-version fixtures can also exercise compatibility with released images |

Version resolution and release preflight run first. If either fails, downstream
validation and image builds are skipped. After preflight succeeds, Rust
lint/generated-contract checks, Rust unit/database tests, frontend tests,
documentation, and candidate image builds run in parallel.
Stable and development publication still require every
validation, candidate, and documentation job to succeed. The `Rust validation`
check aggregates both Rust jobs so existing required-check settings keep covering
the complete Rust suite.

System prerequisites already installed on the runner are reused without contacting
APT mirrors. When packages are missing, the shared installer uses bounded network
retries and timeouts, with a three-minute limit for each APT update or install.
An unavailable mirror therefore fails the prerequisite step promptly instead of
consuming the entire build timeout.

Each existing GHCR package must grant the workflow repository Write access under
**Package settings > Manage Actions access**; a successful registry login alone
does not establish package access. Both Skopeo and Docker are authenticated so
image transfer and Cosign signing can access private packages.

Rust caches retain workspace crates as well as dependencies, with separate caches
for checks, tests, and each candidate architecture. Database regressions use the
same workspace feature resolution as the unit tests to reuse their compiled
executables. Clippy checks all workspace targets, replacing the separate
`cargo check` pass. Host Cargo jobs use the runner's available CPU count instead
of the two-job limit used for interactive development. The first run with new
cache keys will populate those caches; later runs can reuse them. Parallel jobs
reduce elapsed time but may consume more
total runner minutes, especially when an early check fails.

Candidate jobs also persist Docker's Cargo cache mounts explicitly; the Docker
layer cache alone does not retain them. Registry sources, release compilation,
and the volume helper are restored into the same Buildx builder used for both
images. Compatibility-test compilation has a separate retained directory and
continues to run inside the pinned Rust builder. Both caches are separated by
architecture and build configuration, restore compatible earlier commits, and
save a new snapshot for each commit. Cache misses rebuild normally; cache hits
still run Cargo and all packaged tests. The first run populates these caches.

Agent jobs reclaim unused SDK space on disposable GitHub-hosted runners before building
both production images. The cleanup is skipped on self-hosted runners. Database
fixtures must use unique Docker service IDs and bootstrap token hashes because
most regression suites share one database and the production schema enforces
uniqueness. Platform HTTP tests use an isolated database per fixture.

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
