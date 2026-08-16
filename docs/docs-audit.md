# Citadel documentation audit

This audit is an implementation artifact. It is intentionally outside the
public Fumadocs content tree.

## Existing public documentation

| Source | Topic | Status | Evidence | Action | Destination |
| --- | --- | --- | --- | --- | --- |
| `docs/user/access-control.md` | Users, Teams, Roles, and resource access | Incomplete | Identity handlers, permission tests, Service Account specification | Rewrite links and retain | `concepts/access-control.md` |
| `docs/user/adopting-existing-workloads.md` | Container, Compose, Swarm Service, and Swarm Stack adoption | Accurate but large | Adoption handlers and integration tests | Retain and normalize links | `guides/adopting-existing-workloads.md` |
| `docs/user/agent.md` | Regular Agent installation and lifecycle | Accurate | Agent transport implementation and compatibility tests | Retain | `operations/agent.md` |
| `docs/user/alert-rules.md` | Alert channels and rules | Accurate | Alert handlers, evaluators, and integration tests | Retain | `guides/alert-rules.md` |
| `docs/user/automation-actions.md` | Automation actions and execution | Accurate | Automation handlers, sandbox, and tests | Retain | `guides/automation-actions.md` |
| `docs/user/backups.md` | Backup repositories, policies, runs, and restore | Accurate | Backup specification, handlers, and acceptance tests | Retain | `guides/backups.md` |
| `docs/user/builds.md` | Build projects and pools | Accurate | Build handlers and Edge build-agent tests | Retain | `guides/builds.md` |
| `docs/user/control-plane-recovery.md` | Citadel database and key recovery | Accurate with manual restore limitation | Backup implementation and recovery acceptance tests | Retain and identify manual recovery limitation | `operations/control-plane-recovery.md` |
| `docs/user/deployments.md` | Single-container managed workloads | Accurate | Deployment handlers, domain model, and integration tests | Retain | `guides/deployments.md` |
| `docs/user/docker-compose-configuration.md` | Production environment configuration | Outdated image-tag default | Compose files, `.env.example`, and release procedure | Rewrite release-tag guidance | `getting-started/configuration.md` |
| `docs/user/docker-swarm.md` | Swarm platforms and managed resources | Accurate but broad | Swarm specifications and integration/acceptance tests | Retain | `concepts/docker-swarm.md` |
| `docs/user/edge-agent.md` | Edge Agent enrollment and lifecycle | Accurate | Edge transport and enrollment tests | Retain | `operations/edge-agent.md` |
| `docs/user/first-run-setup.md` | Initial administrator setup | Accurate | Setup handlers and tests | Retain | `getting-started/first-run-setup.md` |
| `docs/user/git-repositories.md` | Git accounts and repository sync | Accurate | Git repository handlers and compatibility tests | Retain | `guides/git-repositories.md` |
| `docs/user/git-stacks.md` | Git-backed Stack operation | Accurate | Stack and Git integration tests | Retain | `guides/git-stacks.md` |
| `docs/user/licensing.md` | Community and Team entitlements | Accurate for implemented gates | License service and entitlement tests | Retain without unreleased commercial claims | `overview/licensing.md` |
| `docs/user/oidc-providers.md` | OIDC configuration | Accurate | OIDC handlers and compatibility tests | Retain | `guides/oidc-providers.md` |
| `docs/user/platform-monitoring.md` | Platform metrics and disk usage | Accurate | Stats jobs and integration tests | Retain | `operations/platform-monitoring.md` |
| `docs/user/registries.md` | Registry configuration | Accurate | Registry handlers and connector tests | Retain | `guides/registries.md` |
| `docs/user/resource-tags.md` | Resource tagging | Accurate | Tag handlers and UI tests | Retain | `concepts/resource-tags.md` |
| `docs/user/service-accounts.md` | Machine identities and tokens | Accurate | Service Account specification and integration tests | Retain | `concepts/service-accounts.md` |
| `docs/user/tls-and-secure-agent-transport.md` | TLS, proxy, Agent, and Edge transport | Accurate | Transport implementation and tests | Retain | `operations/tls-and-secure-agent-transport.md` |
| `docs/user/two-factor-authentication.md` | MFA enrollment and policy | Accurate | Authentication handlers and tests | Retain | `guides/two-factor-authentication.md` |
| `docs/user/variables-and-secrets.md` | Bindings and secret providers | Accurate | Binding resolution and Vault tests | Retain | `concepts/variables-and-secrets.md` |
| `docs/user/web-editor-stacks.md` | Manual Stack authoring | Accurate | Stack form and apply tests | Rename for task-oriented navigation | `guides/manual-stacks.md` |
| `docs/user/webhooks.md` | Git and shared-secret webhooks | Accurate | Webhook listener and authorization tests | Retain | `guides/webhooks.md` |

## Missing release-critical pages

| Topic | Status | Evidence | Action | Destination |
| --- | --- | --- | --- | --- |
| Product introduction | Missing | Current resource model and public user guides | Add concise overview | `index.mdx` |
| Local quick start | Missing | Compose configuration and first-workload flows | Add one path from startup through the first Deployment | `getting-started/quick-start.mdx` |
| Architecture | Missing | Core, Agent, Edge Agent, PostgreSQL, and Docker connectors | Add boundary-focused explanation | `overview/architecture.mdx` |
| Requirements and compatibility | Unresolved | Docker API version, .NET/Node build settings, Agent protocol tests | Publish only confirmed minimums; record unresolved Docker/browser matrix | `getting-started/requirements.mdx` |
| Clean installation | Missing | Compose files and first-run setup | Add supported pre-release source-build path and stable-tag path | `getting-started/install.mdx` |
| First Platform | Missing | Platform forms and handlers | Add short onboarding path | `getting-started/first-platform.mdx` |
| First Deployment | Missing | Deployment guide | Add minimal first workload tutorial | `getting-started/first-deployment.mdx` |
| First Stack | Missing | Manual Stack guide | Add minimal Compose tutorial | `getting-started/first-stack.mdx` |
| Upgrade and rollback | Missing | Stable release procedure and migration startup | Add exact-tag workflow and database warning | `operations/upgrade-and-rollback.mdx` |
| Troubleshooting index | Missing | Existing per-feature troubleshooting sections | Add routing page with diagnostics and support data rules | `operations/troubleshooting.mdx` |
| Uninstallation | Missing | Compose persistence model | Add explicit destructive boundaries | `operations/uninstallation.mdx` |
| Environment-variable reference | Incomplete | `.env.example` and configuration models | Generate/maintain concise public reference | `reference/environment-variables.mdx` |
| Port and traffic matrix | Missing | Transport settings and Agent docs | Add directional matrix | `reference/network-ports.mdx` |
| Permission matrix | Missing | Permission matrix API and domain enums | Add user-facing matrix or link to UI | `reference/permissions.mdx` |
| Known limitations | Missing | User docs and accepted v1 limitations | Add only confirmed limitations | `reference/known-limitations.mdx` |
| Release information | Missing | `version.json`, stable release procedure, release workflow | Add pre-release state and tag policy | `reference/releases.mdx` |
| Public API reference | Resolved as preview | Explicit endpoint metadata, filtered schema generation, and schema contract tests | Link to the independently hosted ReDoc site | `reference/api.mdx` |

## Blocking discrepancies

1. The production Compose default no longer selects unpublished `latest`; a
   release installation must set an exact version or digest.
2. The repository currently contains version `1.0.0` but has no stable `v*`
   release tag. Public pages must say pre-release until the first stable tag is
   published.
3. No final documentation or API-reference hostname is configured.
4. A supported Docker/Swarm compatibility range is not yet expressed as a
   public release contract.
5. The public OpenAPI subset is now explicit but remains an API Preview until a
   compatibility and deprecation policy is approved.
