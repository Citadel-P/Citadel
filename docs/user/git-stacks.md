# Git Stacks

Git stacks let Citadel deploy Docker Compose projects from a Git repository. They are useful for a single repository with one compose file and for monorepos that hold many independent compose projects.

Configure the Git repository and any required Git account before creating a Git stack. See `docs/user/git-repositories.md`.

Use deployments instead when the workload is a single Docker container and does not need Compose. See `docs/user/deployments.md`.

Use web editor stacks instead when the Compose YAML should be stored and edited directly in Citadel. See `docs/user/web-editor-stacks.md`.

## Simple Repository

Use this setup when the repository contains one compose project.

Example layout:

```text
compose.yml
.env
```

Recommended stack configuration:

- Source: `Git`
- Repository: the linked Git repository
- Branch: select one of the synced repository refs, such as `main` or `master`
- Commit: empty, unless you want to pin this stack to one immutable commit
- Compose paths: `compose.yml`
- Compose env files from repo: `.env`, if the compose project uses it
- Working directory: empty, or `.`
- Watch paths: empty

With watch paths empty, Citadel watches the compose file, repo env files, and the inferred compose working directory. When the repository branch moves to a new commit, Citadel compares the running release commit with the branch head and marks the stack outdated only when relevant files changed.

Path rules:

- Paths are relative to the repository root.
- Absolute paths are rejected.
- `..` path traversal is rejected.
- Compose paths and repo env file paths must point to files.
- Watch paths may point to a file, a directory prefix, or a `path/**` subtree.
- Empty watch paths are usually best unless the stack depends on shared files outside its compose directory.

If the repository has already been synced, the stack form shows branch refs from the repository cache. It also shows the current deployed commit and the latest synced commit for the selected branch, so you can tell whether the stack is behind before deploying.

Use **Discover compose projects** to scan the selected repository branch. For a simple repository, choose the discovered root project to pre-fill compose paths, working directory, repo env files, and watch paths.

## Monorepo

Use this setup when one repository contains multiple compose projects.

Example layout:

```text
stacks/
  beszel/
    compose.yml
    .env
  caddy/
    compose.yml
shared/
  networks.yml
```

Create one Citadel stack per Compose project:

Beszel stack:

- Compose paths:
  - `stacks/beszel/compose.yml`
  - `shared/networks.yml`, if the compose file depends on it
- Compose env files from repo:
  - `stacks/beszel/.env`
- Working directory:
  - `stacks/beszel`
- Watch paths:
  - empty for the default, or explicitly:
  - `stacks/beszel/**`
  - `shared/networks.yml`

Caddy stack:

- Compose paths:
  - `stacks/caddy/compose.yml`
- Working directory:
  - `stacks/caddy`
- Watch paths:
  - empty, or `stacks/caddy/**`

This keeps unrelated monorepo commits quiet. A change under `stacks/caddy` should not mark the Beszel stack outdated unless Beszel explicitly watches that path.

For monorepos, use **Discover compose projects** after selecting the repository and branch. Citadel scans common Compose file names such as `compose.yml`, `compose.yaml`, `docker-compose.yml`, and override files in the same folder. Select the project you want this stack to represent. A Citadel stack still maps to one Compose project; create another stack for another discovered project.

Compose path order matters. Put the base compose file first and override files after it:

```text
stacks/beszel/compose.yml
stacks/beszel/compose.override.yml
```

## Update Policy

For branch-tracking Git stacks, Citadel stores the exact deployed commit in the stack release source metadata.

- `Notify`: mark the stack outdated and emit a Git update alert.
- `StackAutoDeploy`: automatically reapply the stack when relevant Git paths change.
- `ServiceAutoDeploy`: currently treated as stack-level Git source deployment because a Git commit can affect networks, volumes, env files, and dependencies.
- `Disabled`: do not report Git source updates.

`Notify` and update detection remain available in Community. Automatic
deployment caused by continuously observed repository changes requires
`Operational Guardrails`. Deployment caused by an external webhook requires
`Automated Operations`.

Pinned stacks set `Commit` to a SHA. They do not track branch updates and webhook deploys are ignored.

## Build Images

Use **Build Images** when a Compose service should use an image produced by a Citadel build project.

For each service binding:

- Compose Service: exact service name from the Compose files
- Build: build project that produces the service image
- Redeploy On Build: automatically reapply that service after the selected build succeeds

`Redeploy On Build` requires `Automated Operations`.

When the stack is applied, Citadel uses the desired artifact stored on each binding and writes a generated Compose override file. A binding without a resolved artifact falls back to the latest successful build. The override is added after the repository Compose files so the build image replaces the service image from Git without changing the repository.

You can save bindings before the first successful build, but apply fails until each selected build has a successful image.

When a mapped build succeeds later, Citadel updates the binding's desired image reference and digest. If `Redeploy On Build` is enabled, Citadel reapplies only the mapped service. Applied state changes only after that apply succeeds.

For build setup and webhook-triggered builds, see `docs/user/builds.md`.

## Webhooks

Repository webhooks trigger repository sync. Git stack webhooks trigger deploy for that stack.

Repository synchronization and pending-update detection remain Community.
Applying the stack from the webhook requires `Automated Operations`.

For the shared listener model, authentication options, URL shape, and troubleshooting, see `docs/user/webhooks.md`.

For Git stacks, webhook deploy still validates:

- provider authentication, if a secret is configured
- repository identity
- branch filter
- stack is Git-backed
- stack is not pinned
- changed paths, when the provider payload includes them

If the provider payload includes changed paths and they only touch unrelated monorepo paths, Citadel records a webhook activity as a no-op and does not deploy.

If the provider payload does not include changed paths, Citadel syncs the repository branch, diffs the running release commit against the new branch head, and applies the same watch-path decision. This keeps Git providers or proxy payloads that omit file lists usable without blindly redeploying every stack in a monorepo.

Webhook activities show whether the request was queued, rejected, or skipped. The activity details include the request id, provider event, delivery id, branch, commit, repository identity, and no-op reason when one exists.

### Forgejo and Gitea

Forgejo and Gitea can use the GitHub-compatible webhook mode.

- In Citadel, enable the GitHub provider webhook and generate or enter a secret.
- In Forgejo or Gitea, use the Citadel webhook URL from the stack or repository form.
- Put the Citadel secret in the provider webhook secret field. Do not put it in the Authorization header.
- Use the push event.

Citadel accepts GitHub `X-Hub-Signature-256` signatures and the GitHub-compatible `X-Gitea-Signature` / `X-Forgejo-Signature` HMAC-SHA256 headers.

## Rollback

Rollback uses the selected healthy release snapshot, not the current branch head. For Git stacks, the rollback release pins:

- repository id
- branch
- resolved commit SHA
- compose paths
- repo env files
- working directory
- watch paths

If the pinned rollback commit is no longer available in the local repository cache or remote history, apply fails cleanly instead of deploying from an ambiguous branch state.

Citadel keeps the current Git source snapshot and snapshots needed by healthy rollback releases. Stale release source folders are pruned after a successful Git stack apply.

When a Git stack apply fails, Citadel leaves the current source pointer unchanged and discards the snapshot created for that failed attempt.
