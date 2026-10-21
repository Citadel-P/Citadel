---
title: "Git repositories and accounts"
description: "Connect Git repositories and credentials for Stacks and Builds."
---

Git repositories let Citadel sync source-controlled files into a local repository cache. Git stacks use that cache to deploy Docker Compose projects from branches, commits, and paths inside the repository.

Git accounts store reusable credentials for Git hosts. Create a Git account when the repository is private or when the Git provider requires authenticated clone and pull access.

Use web editor stacks instead when the Compose YAML should be stored directly in Citadel and does not need a Git workflow. See [Web Editor Stacks](/docs/resources/stacks/web-editor).

Use Git stacks after the repository has been added and synced. See [Git Stacks](/docs/resources/stacks/git).

Use builds when a Dockerfile in the repository should produce an image pushed to a registry. See [Builds](/docs/resources/builds).

## Connect a repository

Ask the application's maintainer for its clone URL and branch name. For a private
repository, also prepare a read-only token or SSH deploy key.

1. Open **Repositories**.
2. For private access, create the matching credentials in **Git Accounts** first.
3. Add the repository, enter its URL and default branch, and choose the account if needed.
4. Select **Save** and wait for the initial sync to finish.
5. Check that the repository files and branches are available before creating a Git Stack or Build Project.

Optional [On Clone / On Pull hooks](#clone-and-pull-hooks) run on Core during
synchronization. Review any configured commands before saving or synchronizing.

## Git Accounts

A Git account represents credentials for one Git host or domain.

Create an account from the Repositories page, in the **Git Accounts** section, and choose:

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
- On Pull / On Clone: optional shell commands executed on Core during synchronization.
- Webhook: optional provider webhook that queues a repository pull.

Citadel normalizes repository URLs by trimming whitespace, a trailing slash, and a trailing `.git` suffix.

When a Git account is linked, the repository URL domain must match the account domain. For example, an account with domain `github.com` can be linked to `https://github.com/org/repo`, but not to `https://gitlab.com/org/repo`.

## Sync Mode

Citadel keeps a local cache for each repository under:

```text
/app/data/repos/{repository-id}
```

Repository sync updates that cache and records branch refs that Git stacks can select.
Every scheduled sync still fetches the remote. If the fetched commit and checked-out
branch already match and the working tree is clean, Citadel skips checkout and reset.
Changed commits, branch changes, and dirty tracked files use the normal checkout/reset
path. Successful unchanged syncs still update their timestamp and notify waiting consumers.
Citadel also uses the normal path when Git index flags can hide file changes or the
bounded cleanliness checks cannot inspect the complete output.
When index flags hide changes, Citadel rebuilds the cached index from the current
commit before checkout/reset so those flags cannot preserve modified or missing files.

Available sync modes:

- `Pull on interval`: Citadel checks the repository on a schedule. The pull interval is configured in minutes and must be at least `1`.
- `Manual only`: Citadel syncs when you select **Sync Repo** or when another explicit workflow queues a repository sync.

Creating a repository queues an initial sync. Editing the repository URL, default branch, or linked account clears the previous cache and queues a new sync.

Repository webhooks are separate from sync mode. A repository can be manual-only and still accept provider webhooks, or it can use interval polling and webhooks together.

Git Stack update monitoring uses the repository's synchronized branch refs.
A completed sync wakes the Stack checks; stacks sharing a repository do not
start their own periodic fetches. A five-minute reconciliation checks stored
refs if a notification was missed. Explicit Stack update checks, Apply, and
Build runs can still request a fresh synchronization.

Successful background syncs (scheduled checks, webhooks, and Stack/Build cache
refreshes) do not create activity entries, even when the commit changes.
Explicit repository syncs and initial syncs requested by a user remain in the
activity history. Background failures are recorded when the error changes or
recurs after recovery; identical consecutive failures are not repeated.
The **Checks** popover shows the live check status and timestamp independently
of activity history.

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

Repository hooks run in the existing sync worker for manual, interval, webhook,
and Stack/Build-triggered synchronization:

- **On Clone** runs after a fresh clone, before the cache is published. A failed
  hook discards that staged clone. It runs again if the cache is rebuilt.
- **On Pull** runs after an existing checkout is refreshed, including checks
  where the commit has not changed. It does not also run after an initial clone.
- Each command runs sequentially in a separate `/bin/sh -eu -c` process. Shell
  variables and `cd` changes do not carry between commands. Required tools must
  already be installed in Core.
- **Path** defaults to `./` and must resolve to an existing directory inside the
  checkout. Absolute paths, parent traversal, and symlinks escaping the checkout
  are rejected.
- The command list has a shared 120-second execution budget. Each command allows
  at most 64 KiB stdout and 16 KiB stderr. A nonzero exit, timeout, cancellation,
  or output-limit failure stops the remaining commands and fails synchronization.

Write permission is required to change hooks; read-only users can inspect them.
Manual **Sync** requires Execute permission. While a sync is running, changes
to its source or hooks and repository deletion are rejected; metadata can still
be edited. Grant repository Write only to users
trusted to run shell commands as the Core operating-system user. Hooks are not
sandboxed; the working-directory check is not a filesystem access boundary.
Git account authentication is not passed to hook processes, and temporary SSH key
files are removed before hooks start. Core's application environment is not
inherited, though the process runner supplies its normal tool environment such as
PATH and proxy settings. Command text and stdout/stderr are not included in sync
errors; the failed hook, command number, and failure reason are reported.

For repositories with hooks, failed or interrupted synchronization requires
review and a manual **Sync** of the affected branch before automatic refreshes
resume. Polling, webhooks, and deployment refreshes cannot retry that failed
branch automatically. Already completed command side effects are not rolled
back. Make commands safe to repeat, inspect external side effects before retrying,
and do not assume exactly-once execution. Disconnecting the browser does not
cancel the durable sync; Core shutdown cancels its processes.

Hook-created or modified working-tree files do not become Stack deployment inputs:
Git Stacks still materialize the selected **committed Git tree**. Commit required
inputs or generate them in CI. Stack pre-deploy/post-deploy commands remain a
separate deployment feature.

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

Use **Discover compose paths** in the stack form to scan the repository branch and select Compose files. Discovery also suggests environment files alongside the Compose files. Configure working directory and watch paths separately when their defaults do not fit your project.

## Using Repositories With Builds

Builds use Git repositories as Docker image sources.

After the repository is added, create a build and choose:

- Repository: the Git repository that contains the Dockerfile.
- Branch: a discovered branch from that repository.
- Context: Docker build context path relative to the repository root.
- Dockerfile: Dockerfile path relative to the repository root.

Builds sync the selected branch before each run and store the resolved commit SHA on the run.

For build setup, see [Builds](/docs/resources/builds).

## Common Setups

Public HTTPS repository:

- Do not create a Git account.
- Use an HTTPS repository URL.
- Leave Git Account empty.

Private HTTPS repository:

- Create a Git account with transport `HTTPS`.
- Use auth type `Token` unless your Git provider specifically requires basic credentials.
- Link that account to the repository.

### Private SSH repository

1. Create a dedicated, read-only deploy key for the repository. Unattended sync
   does **not** support passphrase-protected private keys; leave passphrase empty.
   Store the private key only in the Citadel Git Account, using **SSH / SSH Key**.
2. Obtain the Git server's public host key and verify its fingerprint against
   provider-published fingerprints or another trusted channel. An unverified
   `ssh-keyscan` result is not trusted evidence.
3. Put that verified host-key entry in a file readable by Core. The standard
   installation uses `/app/data/git-known-hosts` (`<data root>/git-known-hosts`
   with a custom data root). For a separate read-only file, create
   `git-known-hosts` beside the installation Compose file and use this overlay:

   ```yaml
   services:
     server:
       environment:
         CITADEL_GIT_KNOWN_HOSTS_PATH: /etc/citadel/git-known-hosts
       volumes:
         - ./git-known-hosts:/etc/citadel/git-known-hosts:ro
   ```

   The source must be an existing file, not a directory. A verified entry has
   the form `git.example.com ssh-ed25519 <verified-public-host-key>`; replace the
   placeholder with the actual verified public key. For a nonstandard SSH port,
   use `[git.example.com]:2222` as the known-hosts hostname.
4. Recreate Core with the overlay, preserving its existing volumes and project
   name. Use `git@git.example.com:team/app.git`, link the matching Git Account,
   save, and select **Sync Repo**. Confirm the sync succeeds and **Browse Repo**
   shows the expected committed files.

Citadel enforces strict host verification and non-interactive SSH. Never disable
host-key checking to work around a missing or mismatched entry.

## Troubleshooting

If repository sync fails:

- Confirm the repository URL can be cloned from the Citadel host.
- Confirm the default branch exists.
- For a linked account, confirm the account domain matches the repository URL domain.
- For HTTPS private repositories, confirm the token has read access.
- For SSH host-verification failures, check the known-hosts path, Core read access, hostname/port, and verified fingerprint; investigate changed keys before replacing them.
- For SSH authentication failures, check the deploy key, repository permissions, username and linked account domain.
- For an unsupported-passphrase error, use a dedicated unattended deploy key without a passphrase; changing known-hosts entries does not fix credentials.
- Review the repository activity entry for the exact clone or pull error.

If a branch does not appear in a Git stack form, sync the repository first. You can still enter a branch manually when the ref has not been cached yet.
