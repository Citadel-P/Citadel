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

## Release documentation publication

Both annotated stable tags (`vMAJOR.MINOR.PATCH`) and numbered development tags
(`vMAJOR.MINOR.PATCH-dev.N`) publish the retained static documentation artifact to
GitHub Pages after the release checks and image promotion succeed. Branch/PR builds
validate documentation without publishing it. Both publication jobs use the same
Pages action and `github-pages` environment; that environment must allow release tags.
`DOCS_SITE_URL` supplies the canonical HTTPS URL for builds and publication checks.

The shared docs site follows the highest published SemVer across both channels:
`0.1.0-dev.9 < 0.1.0-dev.10 < 0.1.0 < 0.1.1-dev.1`. Older tag retries skip replacing
the site, while their retained release documentation remains available. Consequently,
the shared site can describe development features before the stable demo has them.
Publication holds the same concurrency lock through Pages deployment and release
finalization. Failed Pages deployment leaves the release recoverable and prevents
its downstream VPS deployment. Eligible releases cannot finalize without successful
publication to the configured docs URL.

## Release deployment checks

The release tests cover deployment after successful publication, selection of one
release channel per target, exact Core and paired Agent digests, stale aliases,
legacy state migration, failed-attempt ordering, and HTTPS health verification.
The installation Compose file starts Core and PostgreSQL; the paired Agent digest
configures Core's Agent provisioning image, rather than starting an Agent service.

Publication and demo deployment share the `citadel-product-publication` concurrency
group through the public health check. GitHub's `queue: max` retains up to 100
pending jobs, with cancellation of running jobs disabled. Deployment still checks
alias ownership and VPS attempt history because job arrival order is not version
order. Separate repository toggles enable the two targets:
`CITADEL_PREVIEW_DEPLOY_ENABLED` selects development deployment to the `preview`
environment, while `CITADEL_DEMO_DEPLOY_ENABLED` selects stable deployment to the
`demo` environment. Each job has a fixed channel and waits for its corresponding
publication job. Connection settings and SSH secrets are scoped to each environment;
the previous shared `CITADEL_DEMO_CHANNEL` selector is retired.

The targets use separate Compose projects, host ports, databases, networks, volumes,
and deployment state. Default memory ceilings are 1 GiB Core + 512 MiB PostgreSQL
for preview and 2 GiB Core + 1 GiB PostgreSQL for stable. CPU ceilings also limit
preview load. Operators can tune these values in each server-owned `.env` through
`CITADEL_CORE_MEMORY_LIMIT`, `CITADEL_DATABASE_MEMORY_LIMIT`, `CITADEL_CORE_CPUS`,
and `CITADEL_DATABASE_CPUS`. Compose rendering tests verify target separation and
resource limits without starting containers.

Run the release regression suite with
`python3 -m unittest discover -s test/release -v`. The tests use the Python standard
library and do not require VPS or publication access. Actionlint 1.7.12 does not
recognize the newer [`concurrency.queue` setting](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#concurrency);
validate that setting against GitHub's schema when using this version, and retain
all other workflow diagnostics.

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
