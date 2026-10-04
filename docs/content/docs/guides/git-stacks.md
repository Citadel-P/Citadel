---
title: "Git Stacks"
description: "Deploy and operate Docker Compose or Swarm Stack definitions sourced from Git."
---

Git stacks let Citadel deploy Docker Compose projects from a Git repository. They are useful for a single repository with one compose file and for monorepos that hold many independent compose projects.

Configure the Git repository and any required Git account before creating a Git stack. See [Git repositories and accounts](/docs/guides/git-repositories).

Use deployments instead when the workload is a single Docker container and does not need Compose. See [Deployments](/docs/guides/deployments).

Use web editor stacks instead when the Compose YAML should be stored and edited directly in Citadel. See [Manual Stacks](/docs/guides/manual-stacks).

To import a Compose project that is already running and use a Git repository
as its authoritative source, see
[Adopt existing workloads](/docs/guides/adopting-existing-workloads).

## Deploy from a repository

You need a connected Platform and a repository that has synced successfully in
**Repositories**. Ask the application's maintainer for its Compose file path.

1. Open **Stacks**, select **Add**, and choose **Git** as the source.
2. Select the Platform, repository, and branch.
3. Select the Compose file and any environment files that belong to the application.
4. Review missing variables, secrets, and editor errors.
5. Select **Save**, then **Deploy**. Check the **Services** tab and the deployed release.

Pulling new Git code does not automatically deploy it unless an automatic update
policy is enabled. Start with manual deployment while you check the configuration.

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

Use **Discover compose paths** to scan the selected repository branch and select the Compose files for this stack. Discovery suggests paths; it does not create stacks or automatically select all discovered files.

## Inspecting Deployed Source

After a Git Stack is saved, its Config tab includes **Source files**. This panel
shows:

- the linked repository and branch;
- the exact deployed commit;
- the latest synchronized commit for that branch;
- Compose and repository env files used by the deployed release.

Before the first deployment, the panel can browse the latest synchronized
revision from the saved repository and branch. Deployed source paths and
comparison become available after the first successful deployment.

Use **Browse deployed source** to inspect the immutable revision that produced
the current Stack. Use **Browse latest source** to inspect the latest locally
synchronized revision without applying it. Stack Read permission does not grant
repository source access; Git Repository Read permission is also required.

When the deployed and latest commits differ, **Compare** lists repository-wide
path changes and opens supported text files in a read-only diff. Compose and
repository env paths used by the Stack are highlighted. Added, deleted,
renamed, and copied files use the appropriate old and new paths.

The source browser remains bound to persisted source settings while the Stack
form has unsaved repository, branch, commit, Compose path, working directory,
or env-file changes. Save the Stack before browsing the updated configuration.

Browsing and comparison use the local Git object database. They do not
synchronize the repository, run hooks, deploy the Stack, or change update
state. See [Git repositories and accounts](/docs/guides/git-repositories) for content and cache limitations.

## Monorepo

Use this setup when one repository contains multiple compose projects.

Example layout:

```text
stacks/
  demo-app/
    compose.yml
    .env
  caddy/
    compose.yml
shared/
  networks.yml
```

Create one Citadel stack per Compose project:

Demo App stack:

- Compose paths:
  - `stacks/demo-app/compose.yml`
  - `shared/networks.yml`, if the compose file depends on it
- Compose env files from repo:
  - `stacks/demo-app/.env`
- Working directory:
  - `stacks/demo-app`
- Watch paths:
  - empty for the default, or explicitly:
  - `stacks/demo-app/**`
  - `shared/networks.yml`

Caddy stack:

- Compose paths:
  - `stacks/caddy/compose.yml`
- Working directory:
  - `stacks/caddy`
- Watch paths:
  - empty, or `stacks/caddy/**`

This keeps unrelated monorepo commits quiet. A change under `stacks/caddy` should not mark the Demo App stack outdated unless Demo App explicitly watches that path.

For monorepos, use **Discover compose paths** after selecting the repository and branch. Citadel recognizes `.yml` and `.yaml` filenames containing `compose` separated by dots, hyphens, or underscores, such as `docker-compose.yaml`, `sample-app-compose.yaml`, `app-compose-prod.yml`, and `app.compose.yml`. Matching is case-insensitive.

Environment file discovery recognizes `.env`, `.env.production`, `demo-app.env`, and `sample-app.env.local` alongside each project's Compose files. These are suggestions; select only the files needed by the stack.

Separate folders are optional. A Citadel stack maps to one Compose project: when Demo App and Sample App have separate Compose files at the repository root, create two stacks and select the corresponding file for each.

Compose path order matters. Put the base compose file first and override files after it:

```text
stacks/demo-app/compose.yml
stacks/demo-app/compose.override.yml
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

Use **Check for updates** on a Git Stack to query the Git remote immediately
and compare relevant paths with the deployed commit. The
check records update state for that Stack only; it does not apply the Stack,
run repository hooks, emit an alert, or process other Stacks that use the same
repository. It remains available when periodic update behavior is disabled.

Use **Reconcile drift** separately when you need to compare the saved Stack
definition with containers currently running on the Platform.

On the Stacks page, use **Updates available** to show only stacks with a newer
relevant commit or detected service-image digest. The filter works with
search, tags, and the Platform filter.

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

For build setup and webhook-triggered builds, see [Builds](/docs/guides/builds).

## Webhooks

Repository webhooks trigger repository sync. Git stack webhooks trigger deploy for that stack.

Repository synchronization and pending-update detection remain Community.
Applying the stack from the webhook requires `Automated Operations`.

For the shared listener model, authentication options, URL shape, and troubleshooting, see [Webhooks](/docs/guides/webhooks).

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

Automatic-update activity reports the commit actually applied. If the branch
advances between an update check and deployment, this can be newer than the commit
shown by the earlier check.
