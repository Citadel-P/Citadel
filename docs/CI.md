# Continuous integration

Pull requests and branch builds validate code, generated contracts, documentation
and packaged Core/Agent behavior. Contributors can run these checks without
publishing credentials or access to the project's hosting environments.

## Checks on a pull request

| Workflow | Checks |
| --- | --- |
| CI | Pinned native NBGV/version and promotion policy fixtures, digest-transfer testing between two local registries; Rust source ownership and isolated build context; formatting; generated Docker API, database schema, OpenAPI and frontend client; workspace Clippy and tests; database regressions; frontend types and unit tests |
| Documentation | Content and links, site configuration regressions, lint and types, static product export, public schema validation, independent ReDoc export, and smoke checks |
| Reusable Core/Agent candidates | Native amd64 and arm64 validation, both packaged runtime checks, Direct/Edge/live Docker tests and complete Core/Agent acceptance for the current checkout |

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

Rust and Docker compilation caches are separated by job, architecture and build
configuration. Docker Cargo cache mounts are retained explicitly; layer caching
alone is insufficient. Cache hits still run Cargo and all checks. Clippy covers
all workspace targets, and database tests use the unit-test feature resolution.

Disposable GitHub-hosted runners reclaim unused SDK space; self-hosted runners do
not. Shared database fixtures require unique Docker service IDs and bootstrap-token
hashes. Platform HTTP uses child databases; Resource Copy HTTP uses a separate fresh
execution database to avoid unrelated partial inventory rows.

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

Deployment consumes the completed release's exact Core and paired Agent digests.
Compose starts Core and PostgreSQL; the Agent digest configures provisioning rather
than adding an Agent service. Alias ownership, attempt ordering and HTTPS health
checks guard activation.

Publication and demo deployment share the `citadel-product-publication` concurrency
group through the public health check. GitHub's `queue: max` retains up to 100
pending jobs, with cancellation of running jobs disabled. Deployment still checks
alias ownership and VPS attempt history because job arrival order is not version
order. Separate repository toggles enable the two targets:
`CITADEL_PREVIEW_DEPLOY_ENABLED` selects development deployment to the `preview`
environment, while `CITADEL_DEMO_DEPLOY_ENABLED` selects stable deployment to the
`demo` environment. Each job has a fixed channel and waits for its corresponding
publication job. Connection settings and SSH secrets are scoped to each environment.

The targets use separate Compose projects, host ports, databases, networks, volumes,
and deployment state. The deployment helper also supports operator-provisioned
rootless targets: it verifies the deployment account, channel, and pinned Docker
engine ID before changing files or starting containers. Compose explicitly replaces
the host mounts with that account's Docker socket and Docker data directory.
Tests render the merged configuration to reject inherited host-root mounts and
exercise failure when an engine or account does not match. Provisioning and data
migration remain operator tasks; CI does not create Linux accounts or use sudo.
Deployment skips an existing PostgreSQL cluster on an unsupported major version
before stopping Core. The product release remains available.
Core and PostgreSQL have no container memory or swap caps
by default on either target (`0` means unlimited). CPU ceilings remain independent.
Operators can tune these values in each server-owned `.env` through
`CITADEL_CORE_MEMORY_LIMIT`, `CITADEL_DATABASE_MEMORY_LIMIT`, `CITADEL_CORE_CPUS`,
and `CITADEL_DATABASE_CPUS`. Compose rendering checks preserve target separation and these defaults.

Release notes use checksum-pinned git-cliff, without a separate release service.
Public releases contain a manifest and checksums; accepted binaries, documentation
and append-only journals stay in internal drafts until verified publication succeeds.
The publisher then removes the draft; the public manifest records completed
publication, so retries skip builds and can resume deployment. Failed publication
keeps its recovery files beyond the 30-day Actions artifact retention. See
[release publication and recovery](release-publication.md) for retention, review
and retry rules.

On Linux amd64, install the pinned notes tool with
`python3 src/tools/release/notes.py install`, then run the existing release suite
with `python3 -m unittest discover -s test/release -v`. The suite needs no VPS or
publication access. Changelog tests use real temporary Git histories and the pinned
binary; they skip locally if it is absent. CI installs it before the existing test
step. Actionlint 1.7.12 does not
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

Follow the [documentation commands](README.md#validation), using Node.js 24 and
`NEXT_PUBLIC_DOCS_URL=http://localhost:3000`. Run type generation and builds
sequentially because both write `docs/.next`. Rebuild with a `/Citadel` URL suffix
and repeat the static smoke test to check Pages subpath hosting.

Rust/frontend commands and generation requirements are in
[Development](DEVELOPMENT.md#checks) and [Tests](../test/README.md).

## License and release compliance

The release-test suite checks the canonical Elastic License 2.0 text, source
and license links, image metadata and known bundled-data notices. These are
baseline regression checks, not a complete third-party license inventory or
legal compliance certification. Follow the
[legal distribution checklist](LEGAL-RELEASE-CHECKLIST.md) before publication.
