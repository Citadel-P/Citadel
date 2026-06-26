# Webhook Listener

## Goal

Allow Git providers to trigger Citadel resource actions without exposing the normal API.

The webhook model should stay resource-owned and simple:

- Git repository webhook triggers repository pull/sync.
- Git stack webhook triggers stack deploy.
- Manual/web-editor stack webhooks are not supported.
- No generic webhook endpoint resource.
- No `WebhookEndpoints` or `WebhookDeliveries` tables.
- No authenticated `/api/v1/webhookEndpoints` API.

## URL Model

Use deterministic listener URLs:

```text
POST /listener/{authType}/{resourceType}/{id}/{execution}
```

Supported values:

| Segment | Values |
| --- | --- |
| `authType` | `github`, `gitlab` |
| `resourceType` | `repo`, `stack` |
| `execution` | repo: `pull`; stack: `deploy` |

Examples:

```text
/listener/github/repo/019ef7a2-0000-7000-9000-000000000000/pull
/listener/gitlab/stack/019ef7a2-0000-7000-9000-000000000001/deploy
```

The listener is anonymous and outside `/api/v1`. Only `/listener/*` needs to be publicly reachable by the Git provider.

## Resource Configuration

Webhook configuration belongs to the resource being triggered.

### Git Repository

Repository webhook configuration uses the existing repository config:

- `WebHookEnabled`
- `WebHookSecret`
- `DefaultBranch`

The repository sync mode controls whether this is the active sync mechanism:

- `PullInterval`: poll on interval
- `Manual`: sync only by explicit user action

Repository webhook enablement is separate from sync mode. A repository can be manually synced, polled on an interval, and/or triggered by provider webhooks.

### Git Stack

Git stack webhook configuration lives in `GitStack` because it is part of the stack configuration and can evolve with releases:

```csharp
public sealed record StackWebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null,
    bool ForceDeploy = false);
```

```csharp
public sealed record GitStack(
    Guid GitRepoId,
    string Branch,
    string? CommitSha,
    StackUpdateBehavior UpdateBehavior,
    string? ProjectName = null,
    StackWebhookConfig? Webhook = null,
    ...);
```

Do not add `Stack.WebHookSecret` or a separate webhook table.

## Authentication

Supported authentication:

- GitHub-compatible HMAC SHA-256 via `X-Hub-Signature-256`
- GitLab signed webhook via `webhook-id`, `webhook-timestamp`, `webhook-signature`
- GitLab legacy token via `X-Gitlab-Token`

Gitea and Forgejo should use the GitHub-compatible path.

The secret is optional for now. When a secret is configured, the listener validates the provider signature/token using the secret from the target resource configuration. When the secret is empty, Citadel accepts the delivery without provider authentication and relies on the unguessable resource ID, route shape, branch filter, repository identity checks, and rate limiting.

## Branch Filtering

Webhook push events should only trigger work for the configured branch.

Defaults:

- Git repository: `GitRepository.DefaultBranch`
- Git stack: `GitStack.Webhook.BranchFilter ?? GitStack.Branch`

Branch mismatch returns accepted no-op and should not generate noisy alerts.

Pinned Git stacks should ignore deploy webhooks because a pinned commit is intentionally immutable.

## Dispatch

The listener must:

1. Parse route and resolve the target resource.
2. Return `404` if the resource is missing or webhook is disabled.
3. Read the raw body with a strict size limit.
4. Validate provider authentication.
5. Parse payload branch, commit SHA, and repository identity.
6. Validate branch and repository identity.
7. Dispatch through existing Citadel behavior.

Repo pull:

- Mark repository as processing.
- Enqueue existing Git repository sync job with trigger `Webhook`.

Stack deploy:

- Require Git stack.
- Require unpinned stack.
- Verify linked repository matches payload when repository metadata is present.
- Use existing stack apply service.
- Do not duplicate lifecycle logic in the listener.

## Activities And Alerts

Webhook deliveries should always leave a small activity trail when Citadel can associate the route with a supported resource type:

- `GitRepoWebhookReceived`
- `StackWebhookReceived`

Activities are the audit log for all accepted, rejected, queued, and no-op deliveries. They may include request id, provider event type, delivery id, branch, commit SHA, repository full name, status, and reason.

Alerts are reserved for user-actionable failures. Do not alert for every successful webhook, every branch mismatch, or every unsupported provider event. Those cases are normal operational noise and should remain activities only.

Minimal alert events:

| Alert type | Resource | When |
| --- | --- | --- |
| `WebhookAuthenticationFailed` | Webhook | A configured webhook target is found, a secret is configured, and provider authentication fails. |
| `WebhookDispatchFailed` | Webhook | The webhook is authenticated but Citadel refuses to dispatch because the configuration or payload is inconsistent. Examples: repository identity mismatch, missing payload branch, pinned Git stack, missing linked repository. |
| `WebhookGitRepoSyncFailed` | Webhook | A repo webhook successfully queues a sync, but the Git sync job fails. |
| `WebhookStackGitDeployFailed` | Webhook | A stack webhook successfully dispatches deploy, but Git stack materialization or stack apply fails. |

Branch mismatch and unsupported provider event type should not emit alerts. They are expected no-op outcomes when providers send broad event traffic or users intentionally filter branches.

## UI

Do not show a generic Webhooks page or endpoint table.

Git repository config:

- Sync mode only controls Citadel-initiated sync: `Manual` or `PullInterval`.
- Webhook settings live in Advanced and can coexist with either sync mode.
- User can enable/disable, set/generate secret, select provider/auth for URL display, and copy URL.

Git stack config:

- In Git Stack settings, show webhook settings.
- User can enable/disable, select provider/auth, set/generate secret, configure branch filter, and copy URL.
- Manual/web-editor stacks do not show webhook settings.

The stack webhook form writes to `GitStack.Webhook` and participates in normal stack config save/preview.

## Tradeoffs

This simplified model intentionally drops durable delivery history and DB-level dedupe. Duplicate provider deliveries may enqueue duplicate work, but existing resource processing locks should prevent conflicting execution. Delivery audit can be added later if operational evidence shows it is needed.
