---
title: "Git Stacks"
description: "Deploy and operate Docker Compose or Swarm Stack definitions sourced from Git."
---

Git stacks deploy Compose definitions from a Git repository as Docker Compose
projects on Standalone Platforms or native Stacks on Swarm Platforms. They
support repositories with one application and monorepos containing several
independent applications.

Configure the Git repository and any required Git account before creating a Git stack. See [Git repositories and accounts](/docs/resources/git-repositories).

Use deployments instead when the workload is a single Docker container and does not need Compose. See [Deployments](/docs/resources/deployments).

Use web editor stacks instead when the Compose YAML should be stored and edited directly in Citadel. See [Web Editor Stacks](/docs/resources/stacks/web-editor).

To import a Compose project that is already running and use a Git repository
as its authoritative source, see
[Adopt existing workloads](/docs/guides/adopting-existing-workloads).

## Deploy from a repository

You need a connected Platform and a repository that has synced successfully in
**Repositories**. Ask the application's maintainer for its Compose file path.

1. In **Repositories**, [connect the repository](/docs/resources/git-repositories#connect-a-repository)
   and confirm its initial sync succeeds. For private source, configure its Git Account first.
2. Open **Stacks**, select **Add Stack**, enter a name, and choose **Git** as the source.
3. Select the Platform, repository, and branch. Leave **Commit** empty to track
   that branch, or enter a commit SHA for a fixed revision.
4. Select the application's Compose paths in order: base file first, then
   overrides. Select any repository environment files it requires. The
   [simple repository](#simple-repository) example below shows the minimum setup.
5. Review missing variables and secrets on **Bindings** and fix reported errors.
   Keep update behavior **Disabled** or **Notify Only** while validating the first deployment.
6. Select **Save**, then **Deploy**. Wait for the operation to finish and check
   the **Services** tab, application logs, and application endpoint.
7. Record the deployed commit from **Config → Source files** or the release's
   source details. This is the revision to use if you need to return to this deployment.

Pulling new Git code does not automatically deploy it unless an automatic update
policy is enabled. Start with manual deployment while you check the configuration.

## Review and deploy an update

Use this workflow with **Disabled** or **Notify Only** update behavior and no
deploy webhook. Repository sync runs any configured hooks and can trigger other
Stacks whose automatic deployment policies are enabled.

1. Commit and push the application change to the selected branch.
2. Synchronize the repository in **Repositories** to refresh its cached source.
   On the Stack, select **Check for updates** to record relevant changes for that
   Stack. The check itself does not deploy or run repository hooks.
3. In **Config → Source files**, compare the deployed and latest synchronized
   source. **Compare** and source browsing also require Git Repository Read access.
4. Review changed Compose paths, bindings, and images. Save any Stack configuration
   edits, then select **Deploy** or **Redeploy** when ready.
5. Check the operation result, service health, and application behavior. Confirm
   the deployed commit in source details after the operation succeeds.

A branch can advance between the check and deployment. To apply precisely the
revision you reviewed, set **Commit** to its SHA and save before deploying.
A pinned Stack does not track later branch updates until you clear **Commit**.
For recovery, follow [Rollback](#rollback); a saved release alone does not pin
an otherwise unpinned branch to its historical commit.

[![Git Stack configuration showing repository, branch, ordered Compose paths, and an environment file](/screenshots/git-stack-source.png)](/screenshots/git-stack-source.png)

Example configuration for a demo Storefront application. The base Compose file
comes before its production override; the environment file belongs to the same
application. Paths are relative to the repository root. Select the image to enlarge it.

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
state. See [Git repositories and accounts](/docs/resources/git-repositories) for content and cache limitations.

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

[![Git Source Paths with the Storefront working directory and watch paths for the application and shared files](/screenshots/git-stack-watch-paths.png)](/screenshots/git-stack-watch-paths.png)

This demo Stack explicitly watches its application folder and `shared/**`.
Add shared paths only when changes there should mark the Stack outdated; leave
**Watch Paths** empty to use the defaults described above.

## Update Policy

For branch-tracking Git stacks, Citadel stores the exact deployed commit in the stack release source metadata.

- **Notify Only**: mark the stack outdated and emit a Git update alert.
- **Auto Deploy Stack**: automatically reapply the stack when relevant Git paths change.
- **Auto Deploy Services**: unavailable for Git Stacks. A Git commit can affect networks, volumes, env files, and dependencies.
- **Disabled**: disable periodic checks; manual checks remain available.

`Notify` and update detection remain available in Community. Automatic
deployment caused by continuously observed repository changes requires
`Operational Guardrails` and `Automated Operations`. Deployment caused by an external webhook requires
`Automated Operations`.

Pinned stacks set `Commit` to a SHA. They do not track branch updates and webhook deploys are ignored.

Use **Check for updates** on a Git Stack to query the Git remote immediately
and compare relevant paths with the deployed commit. The
check records update state for that Stack only; it does not apply the Stack,
run repository hooks, emit an alert, or process other Stacks that use the same
repository. It remains available when periodic update behavior is disabled.

On Docker Standalone, use **Reconcile drift** separately to compare the saved
Stack definition with running containers. Container drift reconciliation is
not available for Swarm Stacks.

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

Service-scoped deployment is supported only for Docker Standalone Stacks.
For a Swarm Stack, leave **Redeploy On Build** disabled and deploy the complete
Stack manually after the desired build image changes.

For build setup and webhook-triggered builds, see [Builds](/docs/resources/builds).

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

Rollback reloads the selected healthy release's saved Stack configuration,
including its repository, branch, Compose paths, and any explicit **Commit**.
The current Rust implementation does not automatically use the deployed commit
recorded in release source metadata. If the selected release was tracking a
branch with **Commit** empty, rollback can deploy that branch's current head.

To redeploy an exact earlier revision:

1. Open **Releases** and find the healthy release you want to recover. Copy its
   deployed commit SHA from source details.
2. In **Config**, set **Commit** to that SHA. Review the repository, Compose paths,
   environment files, and bindings needed by that revision, then save.
3. Select **Deploy** or **Redeploy**. The commit must still be available to Citadel.
4. Verify service health and application behavior, then confirm the deployed
   commit matches the selected SHA.

Keep **Commit** pinned while diagnosing the regression. Clear it and save only
when you intend to resume branch tracking and the configured update policy.

Rollback does not restore application volumes or historical secret values.
Review bindings, image references, and external Docker Secrets or Configs
before deploying an older revision.

Automatic-update activity reports the commit actually applied. If the branch
advances between an update check and deployment, this can be newer than the commit
shown by the earlier check.
