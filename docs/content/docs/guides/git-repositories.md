---
title: "Git repositories and accounts"
description: "Connect Git repositories and credentials for Stacks and Builds."
---

Git repositories let Citadel sync source-controlled files into a local repository cache. Git stacks use that cache to deploy Docker Compose projects from branches, commits, and paths inside the repository.

Git accounts store reusable credentials for Git hosts. Create a Git account when the repository is private or when the Git provider requires authenticated clone and pull access.

Use web editor stacks instead when the Compose YAML should be stored directly in Citadel and does not need a Git workflow. See [Manual Stacks](/docs/guides/manual-stacks).

Use Git stacks after the repository has been added and synced. See [Git Stacks](/docs/guides/git-stacks).

Use builds when a Dockerfile in the repository should produce an image pushed to a registry. See [Builds](/docs/guides/builds).

## Git Accounts

A Git account represents credentials for one Git host or domain.

Create an account from the Git Repositories page, in the **Git Accounts** section, and choose:

- Name: a recognizable label for the credentials.
- Domain: the Git host, such as `github.com`, `gitlab.example.com`, or `gitea.internal`.
- Transport: `HTTP`, `HTTPS`, or `SSH`.
- Auth type: the credential format Citadel should use for clone and pull.

Supported authentication:

| Transport | Auth Type | Use For |
| --- | --- | --- |
| `HTTP` or `HTTPS` | `Basic Auth` | Git hosts that still accept username and password-style credentials. |
| `HTTP` or `HTTPS` | `Token` | Personal access tokens, deploy tokens, or provider tokens. |
| `SSH` | `SSH Key` | SSH clone URLs such as `git@github.com:org/repo.git`. |

SSH transport requires SSH key authentication. HTTP and HTTPS accounts cannot use SSH keys.

For token-based access, prefer a least-privilege token that can read only the repositories Citadel needs. For SSH access, use a deploy key when your Git provider supports it.

## Git Repositories

A Git repository is the source definition Citadel syncs.

Create a repository and choose:

- Name: a unique name used inside Citadel.
- Description: optional notes.
- Tags: optional filters for organizing repositories.
- Repo URL: the Git clone URL, for example `https://github.com/org/repo` or `git@github.com:org/repo.git`.
- Default Branch: the branch Citadel should sync first, such as `main`.
- Git Account: optional credentials for private or authenticated repositories.
- Sync mode: how Citadel refreshes the local cache.
- On Pull: optional shell commands to run after a pull.
- On Clone: optional shell commands to run after the first clone.
- Webhook: optional provider webhook that queues a repository pull.

Citadel normalizes repository URLs by trimming whitespace, a trailing slash, and a trailing `.git` suffix.

When a Git account is linked, the repository URL domain must match the account domain. For example, an account with domain `github.com` can be linked to `https://github.com/org/repo`, but not to `https://gitlab.com/org/repo`.

## Sync Mode

Citadel keeps a local cache for each repository under:

```text
/app/data/repos/{repository-id}
```

Repository sync updates that cache and records branch refs that Git stacks can select.

Available sync modes:

- `Pull on interval`: Citadel checks the repository on a schedule. The pull interval is configured in minutes and must be at least `1`.
- `Manual only`: Citadel syncs only when you click **Sync** or when another explicit workflow queues a repository sync.

Creating a repository queues an initial sync. Editing the repository URL, default branch, or linked account clears the previous cache and queues a new sync.

Repository webhooks are separate from sync mode. A repository can be manual-only and still accept provider webhooks, or it can use interval polling and webhooks together.

## Browsing Repository Source

Open a repository and select **Browse Repo** from the action bar to inspect
source already available in Citadel's local cache. The toolbar identifies the
synchronized default branch and revision. Expand directories lazily and choose
a supported text file to open a read-only preview.

The browser is commit-aware. The revision displayed in the toolbar remains
fixed while you browse, even if another synchronization advances the branch.
Use **Sync Repo** in the repository page action bar before opening the browser
when you need to retrieve the latest remote revision.

Repository Read permission grants access to committed source, including `.env`
files. Limit repository access accordingly. Citadel does not redact committed
files by name.

Browser limitations:

- files are read-only;
- binary and files above the preview limit show metadata instead of content;
- symlinks show their target but are never followed;
- submodules show their pinned commit but cannot be opened;
- Git LFS pointer files are shown as text and their referenced objects are not fetched;
- browsing never performs an automatic clone, fetch, pull, or hook;
- exact provider file links are shown only when Citadel recognizes a safe URL pattern.

An older commit may no longer be present in the cache. Citadel reports that
state instead of displaying another revision or fetching it automatically.

## Clone And Pull Hooks

Use **On Clone** and **On Pull** when the repository needs a small preparation step before Git stacks consume it.

Each hook has:

- Path: the command working directory, relative to the repository root.
- Commands: one command per line.

`On Clone` runs after the repository is cloned for the first time. `On Pull` runs after clone and after later pulls.

Keep hooks short and deterministic. Long build steps are usually better as automation actions or CI jobs, because repository sync should stay fast and predictable.

## Webhooks

Repository webhooks let a Git provider notify Citadel after a push.

Use this when you want Citadel to sync quickly after source changes instead of waiting for the next interval.

Common setup:

- Enable the webhook in the repository form.
- Select the provider and authentication format.
- Generate or enter a secret.
- Save the repository.
- Copy the listener URL into the Git provider.
- Enable the provider's push event.

Repository webhooks queue the normal repository sync path. They do not deploy a stack by themselves; Git stack deployment behavior is configured on the stack.

For webhook URL formats, authentication choices, and branch behavior, see [Webhooks](/docs/guides/webhooks).

## Using Repositories With Git Stacks

After the repository syncs successfully, create or edit a stack and choose:

- Source: `Git`
- Repository: the synced repository
- Branch: one of the repository refs, or a branch name you enter manually
- Commit: optional, only when you want to pin the stack to one immutable commit
- Compose paths: one or more Compose files relative to the repository root
- Compose env files from repo: optional `.env` files relative to the repository root

Use **Discover compose projects** in the stack form when you want Citadel to scan the repository branch and pre-fill Compose paths, working directory, env files, and watch paths.

## Using Repositories With Builds

Builds use Git repositories as Docker image sources.

After the repository is added, create a build and choose:

- Repository: the Git repository that contains the Dockerfile.
- Branch: a discovered branch from that repository.
- Context: Docker build context path relative to the repository root.
- Dockerfile: Dockerfile path relative to the repository root.

Builds sync the selected branch before each run and store the resolved commit SHA on the run.

For build setup, see [Builds](/docs/guides/builds).

## Common Setups

Public HTTPS repository:

- Do not create a Git account.
- Use an HTTPS repository URL.
- Leave Git Account empty.

Private HTTPS repository:

- Create a Git account with transport `HTTPS`.
- Use auth type `Token` unless your Git provider specifically requires basic credentials.
- Link that account to the repository.

Private SSH repository:

- Create a Git account with transport `SSH`.
- Use auth type `SSH Key`.
- Use the SSH clone URL for the repository, such as `git@github.com:org/repo.git`.
- Link that account to the repository.

## Troubleshooting

If repository sync fails:

- Confirm the repository URL can be cloned from the Citadel host.
- Confirm the default branch exists.
- For a linked account, confirm the account domain matches the repository URL domain.
- For HTTPS private repositories, confirm the token has read access.
- For SSH repositories, confirm the private key and optional passphrase are correct.
- Review the repository activity entry for the exact clone or pull error.

If a branch does not appear in a Git stack form, sync the repository first. You can still enter a branch manually when the ref has not been cached yet.


