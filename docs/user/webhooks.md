# Webhooks

Webhooks let external systems notify Citadel through a public listener URL. Citadel uses webhooks to trigger resource-owned actions such as syncing a Git repository, deploying a Git stack, queuing a build, running an automation action, queuing a backup policy, or checking a managed Swarm Service image for updates.

Citadel does not have a separate "Webhook" resource page. Webhook settings live on the resource that will be triggered.

## License Availability

Community can receive and authenticate repository webhooks, synchronize source,
detect changes, and show pending updates.

Team's `Automated Operations` capability is required when a webhook starts a
mutating operation:

- deploy a Git stack
- queue a build
- run an automation action
- queue a backup policy
- check or automatically apply a managed Swarm Service image update
- start a deployment or stack apply

Webhook reception and authentication are not themselves paid. Without
`Automated Operations`, Citadel may accept the request and record or synchronize
the change, but it must not start the mutating operation.

A webhook-triggered build that uses an external Build Pool also requires
`Elastic Build Execution`.

## Listener URL

Webhook URLs use this shape:

```text
https://citadel.example.com/listener/{authType}/{resourceType}/{resourceId}/{execution}
```

Supported URL segments:

| Resource | URL resource type | Execution | Result |
| --- | --- | --- | --- |
| Git repository | `repo` | `pull` | Queue repository sync |
| Git stack | `stack` | `deploy` | Queue or run Git stack update/deploy behavior; execution requires Automated Operations |
| Build project | `build` | `run` | Queue a build run; execution requires Automated Operations |
| Automation action | `automation-action` | `run` | Queue an action run; execution requires Automated Operations |
| Backup policy | `backup-policy` | `run` | Queue a backup run; execution requires Automated Operations |
| Managed Swarm Service | `swarm-service` | `update` | Check the configured external image tag and follow the Service update behavior |

Supported auth types:

| Provider | `authType` |
| --- | --- |
| GitHub-compatible | `github` |
| GitLab | `gitlab` |
| Generic / CI | `generic` |

Examples:

```text
https://citadel.example.com/listener/github/repo/019f0000-0000-7000-9000-000000000001/pull
https://citadel.example.com/listener/github/stack/019f0000-0000-7000-9000-000000000002/deploy
https://citadel.example.com/listener/github/build/019f0000-0000-7000-9000-000000000003/run
https://citadel.example.com/listener/gitlab/automation-action/019f0000-0000-7000-9000-000000000004/run
https://citadel.example.com/listener/github/backup-policy/019f0000-0000-7000-9000-000000000005/run
https://citadel.example.com/listener/generic/swarm-service/019f0000-0000-7000-9000-000000000006/update
```

The listener is outside `/api/v1` and is intentionally public. Only `/listener/*` needs to be reachable by the external provider.

## Authentication

Webhook authentication is configured on the target resource.

Supported authentication formats:

- GitHub HMAC SHA-256
- GitLab signed webhook
- GitLab legacy token
- Generic shared secret (sent in the `Authorization: Bearer` header)

When a secret is configured, Citadel validates the provider signature or token before dispatching the webhook.

GitHub and GitLab-compatible configurations may accept unsigned deliveries when their secret is empty. Generic / CI webhooks always require a non-empty shared secret. This credential is scoped to the configured webhook and is not a Citadel user access token.

### Generic / CI

Use Generic / CI when the caller is not sending GitHub or GitLab signatures. Send the configured secret in the HTTP `Authorization` header; never put it in the URL:

```bash
curl -X POST \
  -H "Authorization: Bearer <shared-secret>" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: release-123" \
  -d '{"branch":"main","commitSha":"abc123","changedPaths":["src/app.cs"]}' \
  https://citadel.example.com/listener/generic/repo/<resource-id>/pull
```

The JSON body is optional. Citadel recognizes `branch`, `commitSha` (or `commit`), `repository`, and `changedPaths` when the target action uses Git metadata. When metadata is omitted, Citadel uses the saved resource identity and configured branch. `Idempotency-Key` is recorded for diagnostics; resource processing locks remain the retry-safety boundary.

### GitHub, Forgejo, And Gitea

Use provider `GitHub` for GitHub webhooks.

Forgejo and Gitea can also use the GitHub-compatible mode when they send an HMAC SHA-256 signature.

Provider setup:

- URL: copy the listener URL from Citadel
- Content type: JSON
- Secret: the Citadel webhook secret
- Event: push

Citadel accepts GitHub `X-Hub-Signature-256` signatures and the compatible `X-Gitea-Signature` and `X-Forgejo-Signature` headers.

### GitLab

Use provider `GitLab` for GitLab webhooks.

GitLab supports two authentication choices in Citadel:

- `GitLab signed webhook`: validates GitLab's signed webhook headers.
- `GitLab token`: validates the legacy `X-Gitlab-Token` header.

Provider setup:

- URL: copy the listener URL from Citadel
- Secret token or signing secret: match the Citadel secret
- Event: push

Use the legacy token mode when you want a simple shared token. Use signed webhook mode when your GitLab version and configuration provide signed webhook headers.

## Branch Filters

Branch filters prevent broad provider events from triggering unrelated work.

If a branch filter is set, Citadel only dispatches push events for that branch. Other branch pushes are accepted as no-op deliveries.

Defaults:

- Git repository webhook: repository default branch
- Git stack webhook: stack webhook branch filter, or the stack branch when the filter is empty
- Build project webhook: build branch
- Automation action webhook: no default branch unless you configure one
- Backup policy webhook: no default branch unless you configure one

For automation actions and backup policies, leave the branch filter empty when a non-Git system is calling the webhook and the payload does not contain a Git branch.

## Git Repository Webhooks

Repository webhooks queue a repository sync.

Use this when a Git provider should notify Citadel after a push instead of waiting for manual sync or polling.

Behavior:

1. Citadel validates the provider, secret, branch, and repository identity when present in the payload.
2. Citadel marks the repository as processing.
3. Citadel queues the normal Git repository sync job with trigger `Webhook`.
4. The repository cache is updated through the same sync path used by manual and scheduled sync.

Repository webhook settings can coexist with manual sync or interval polling.

Common setup:

- Enable Webhook in the Git repository form.
- Select provider and authentication format.
- Generate or enter a secret.
- Save the repository.
- Copy the listener URL into the Git provider.
- Enable the provider's push event.

## Git Stack Webhooks

Git stack webhooks trigger Git-backed stack deployment behavior for a specific stack.

Deploying from a Git stack webhook requires `Automated Operations`. Community
can still receive the webhook, synchronize the repository, and report a pending
update.

Web editor stacks do not support stack deploy webhooks. Use an automation action webhook if you need a custom trigger for a web editor stack.

Behavior:

1. Citadel validates the provider, secret, branch, and repository identity.
2. Citadel rejects pinned Git stacks because the configured commit is intentionally immutable.
3. Citadel checks whether the push affects the stack.
4. Citadel applies the stack update behavior:
   - `Disabled`: no deployment.
   - `Notify`: queue repository sync and mark/update Git source state.
   - `StackAutoDeploy`: deploy the stack when relevant paths changed.
   - `ServiceAutoDeploy`: treated as stack-level Git source deployment.

For monorepos, Citadel uses changed paths from the provider payload when available. If the provider payload does not include changed paths, Citadel syncs the repository and compares the previous deployed commit with the new branch head.

Git stack webhook deploy still validates:

- stack is Git-backed
- stack is not pinned
- branch matches
- repository identity matches, when provider metadata is present
- changed paths are relevant, unless force deploy is enabled

For Git stack setup, see `docs/user/git-stacks.md`.

For Git repository and account setup, see `docs/user/git-repositories.md`.

## Build Webhooks

Build webhooks queue a build project run.

Queueing the run requires `Automated Operations`. If the project uses an
external Build Pool, execution also requires `Elastic Build Execution`.

Use this when a Git provider should build and push an image after a branch update.

Behavior:

1. Citadel validates the provider, secret, branch, and repository identity.
2. Citadel checks that the build project is enabled and has no active run.
3. Citadel checks changed paths against the build context and Dockerfile path.
4. Citadel queues a build run with trigger `Webhook`.
5. The run resolves the configured repository branch to an exact commit, packages the build context, builds on the configured platform, and pushes the configured tags.

Build webhooks use the same one-active-run-per-project rule as manual builds. If a build is already queued, preparing, or running, Citadel rejects the delivery instead of starting a second run.

For monorepos, keep the build context scoped to the service directory. If the provider payload includes changed files, unrelated path changes are accepted as no-op deliveries. If the payload omits changed files, Citadel syncs the repository and diffs the latest successful build commit against the new branch head before deciding.

For build project setup, see `docs/user/builds.md`.

## Automation Action Webhooks

Automation action webhooks queue an action run.

Queueing the action requires `Automated Operations`.

Use this when an external Git provider or another system should trigger a small Citadel automation script.

Behavior:

1. Citadel validates provider authentication and optional branch filter.
2. Citadel queues the action run with trigger `Webhook`.
3. The raw webhook body becomes the run arguments payload.
4. The action runs as the configured `Run As User`.

If the payload is not a JSON object, Citadel wraps it as:

```json
{
  "payload": "raw payload"
}
```

For automation action setup, see `docs/user/automation-actions.md`.

## Backup Policy Webhooks

Backup policy webhooks queue a backup run.

Queueing the backup requires `Automated Operations`.

Use this when another system should trigger backups before a deployment, maintenance window, or external release process.

Behavior:

1. Citadel validates provider authentication and optional branch filter.
2. Citadel checks that the backup policy is enabled.
3. Citadel queues a backup run with trigger `Webhook`.
4. The run uses the policy's configured `Run As User`.

Disabled or archived policies do not run from webhooks.

For backup policy setup, see `docs/user/backups.md`.

## Managed Swarm Service Webhooks

Managed Service webhooks trigger the same bounded Registry digest check used by **Check for updates**. They never issue a blind Docker force update.

- The Service must use an external tagged image and have been deployed once so an applied digest exists.
- `Disabled` update behavior makes the delivery a no-op.
- `Notify only` records and streams whether a newer digest is available.
- `Auto deploy` starts the normal durable Service Apply path only when the digest changed and the required license capabilities are available.
- Build-backed Services use the Build Project webhook instead; digest-pinned images cannot be checked for a newer tag.

Service webhook deliveries create `SwarmServiceWebhookReceived` activities. Applying an available image continues through the normal Service operation and reconciliation activities.

## Activities, Runs, And Alerts

Git repository and Git stack webhooks write webhook activity events:

- `GitRepoWebhookReceived`
- `StackWebhookReceived`
- `SwarmServiceWebhookReceived`

Activities can include request id, provider event type, delivery id, branch, commit SHA, repository name, dispatch status, and no-op reason.

Build webhooks appear in build activities and build run history. Automation action webhooks appear in action run history. Backup policy webhooks appear in backup run history.

Webhook alerts are reserved for failures that need attention:

- `WebhookAuthenticationFailed`: a configured webhook target was found, a secret was configured, and authentication failed.
- `WebhookDispatchFailed`: the webhook authenticated, but Citadel refused dispatch because the payload or configuration was inconsistent.
- `WebhookGitRepoSyncFailed`: a repository webhook queued sync, but the Git sync failed later.
- `WebhookStackGitDeployFailed`: a stack webhook dispatched deploy, but materialization or stack apply failed.

Normal no-op outcomes such as branch mismatch, no relevant path changes, no new commit, or unsupported provider event type do not create alert noise.

## Provider Setup Checklist

Use this checklist for Git providers:

1. Save the Citadel resource first so it has an id.
2. Enable Webhook on the resource.
3. Select the provider and authentication format.
4. Generate or enter a secret.
5. Set a branch filter when only one branch should trigger the resource.
6. Save the resource.
7. Copy the listener URL.
8. Paste the URL into the provider webhook settings.
9. Configure the same secret in the provider.
10. Select the push event.
11. Send a test delivery from the provider.
12. Check Citadel activity, action runs, or backup runs.

For Generic / CI, select that provider, keep the generated secret safe, send it as an `Authorization: Bearer` header, and use the copied `generic` listener URL. A push event setting is not required.

## Troubleshooting

Webhook target not found:

- The resource id in the URL is wrong.
- The webhook is disabled.
- The resource type or execution segment is wrong.
- The stack is not Git-backed.

Webhook authentication failed:

- The provider secret does not match the Citadel secret.
- The wrong provider/authentication format is selected.
- GitLab signed webhook headers are missing or expired.
- The provider is sending a token header while Citadel is configured for signed webhook mode, or the reverse.
- A Generic / CI caller omitted the `Authorization: Bearer` header or used a different token.

Webhook accepted but no work happened:

- The event was not a push event.
- The branch did not match the branch filter.
- A Git stack is pinned to a commit.
- A Git stack push did not change any watched paths.
- The repository payload does not match the linked repository.
- The backup policy is disabled or already has an active run.
- A managed Service has update checks disabled, has not been deployed once, or does not use an external tagged image.

Provider cannot reach Citadel:

- Confirm the public Citadel base URL is correct.
- Confirm `/listener/*` is reachable from the provider.
- Confirm reverse proxy routing allows POST requests to `/listener/*`.
- Confirm request bodies up to 1 MB are allowed.

## Documentation Website Note

These docs are plain Markdown so they can be moved into a docs website later.

If Citadel adopts Docusaurus or Fumadocs, keep each resource guide as a separate page and add a Webhooks section under an Operations or Integrations category. The current file structure already maps cleanly to that kind of navigation.
