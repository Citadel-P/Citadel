# Alert Rules

Alert rules let Citadel record alert events and send notifications when platform, deployment, stack, webhook, or automation conditions match.

Notifications are delivered through notification channels. The Citadel server image includes notification delivery support, so normal Docker installs do not need any extra notification service.

## Before You Start

Channel URLs often contain webhook IDs, tokens, or bot credentials. Treat them as secrets.

Use a separate webhook or bot token for Citadel when the destination service supports it. If the URL is exposed later, you can revoke only the Citadel webhook or token without breaking other integrations.

## License Availability

Community includes:

- notification channel creation, testing, update, enablement, and deletion
- every supported notification destination type
- in-app alert events
- Citadel's seeded system alert rules
- enabling or disabling seeded system rules
- assigning notification channels to seeded system rules
- external delivery when a seeded system rule triggers

Community has no license-enforced notification-channel count limit.

Team's `Advanced Alerting` capability adds:

- custom alert-rule creation
- custom conditions and rule behavior
- quiet hours
- cooldown changes
- threshold and required-match changes
- severity changes
- resource-specific scoping

A seeded system rule is installed by Citadel and owned by the Citadel system
actor. In Community, you can change its enabled status and notification-channel
assignments. Changing its other fields requires Advanced Alerting.

If a Team license expires after its grace period, custom rules pause.
Notification channels remain configured, and seeded system rules continue
sending in-app and external notifications.

## Notification Channels

Open:

```text
Monitoring -> Alert Rules
```

Use the `Notification Channels` section to add destinations such as Discord, Teams, Slack, Telegram, ntfy, Gotify, or a generic webhook.

Create a channel with:

- `Name`: friendly channel name shown in Citadel
- `Type`: destination type, such as `Discord`, `Teams`, or `Generic`
- `Channel URL`: destination URL in Citadel's notification URL format
- `Active`: whether Citadel can send to this channel

After entering the URL, use `Send Test Notification`. Citadel verifies the channel and sends a test message.

## Channel URL Format

The channel URL tells Citadel which service to use and which token, webhook, topic, or chat to send to.

Most URLs follow this shape:

```text
service://credentials@destination/path?options
```

Replace placeholder values such as `<token>`, `<webhook-id>`, and `<chat-id>` with the values from the destination service. If a token contains special URL characters such as `@`, `/`, `?`, `&`, or `#`, URL-encode that value before saving it.

## Configure Common Channels

### Discord

In Discord:

1. Open the server settings.
2. Go to `Integrations -> Webhooks`.
3. Create a webhook for the target channel.
4. Copy the webhook URL.

Discord webhook URLs look like:

```text
https://discord.com/api/webhooks/<webhook-id>/<webhook-token>
```

In Citadel:

- Type: `Discord`
- Channel URL:

```text
discord://<webhook-token>@<webhook-id>
```

### Microsoft Teams

In Teams:

1. Create or open the channel that should receive alerts.
2. Add an incoming webhook or workflow webhook for the channel.
3. Copy the webhook URL.

For Teams, use this Citadel URL format:

```text
teams://<group>@<tenant>/<alt-id>/<group-owner>/<extra-id>?host=<webhook-host>
```

Example:

```text
teams://group@tenant/altId/groupOwner/extraId?host=organization.webhook.office.com
```

Use the group, tenant, path values, and host from the Teams webhook URL.

### Slack

In Slack:

1. Create a Slack app or open an existing app.
2. Enable incoming webhooks.
3. Add a webhook to the target channel.
4. Copy the webhook URL.

Slack webhook URLs look like:

```text
https://hooks.slack.com/services/<token-a>/<token-b>/<token-c>
```

In Citadel:

- Type: `Slack`
- Channel URL:

```text
slack://<token-a>/<token-b>/<token-c>
```

You can include a bot name:

```text
slack://citadel@<token-a>/<token-b>/<token-c>
```

### Telegram

In Telegram:

1. Create a bot with BotFather.
2. Copy the bot token.
3. Add the bot to the target chat, group, or channel.
4. Get the chat ID or channel username.

In Citadel:

- Type: `Telegram`
- Channel URL:

```text
telegram://<bot-token>@telegram?chats=<chat-id>
```

For a public channel username:

```text
telegram://<bot-token>@telegram?chats=@channel-name
```

### ntfy

For a public ntfy topic:

- Type: `Ntfy`
- Channel URL:

```text
ntfy://ntfy.sh/<topic>
```

For a protected ntfy server:

```text
ntfy://:<access-token>@<ntfy-host>/<topic>
```

### Gotify

In Gotify:

1. Create an application.
2. Copy the application token.

In Citadel:

- Type: `Gotify`
- Channel URL:

```text
gotify://<gotify-host>/<application-token>
```

### Generic Webhook

Use a generic webhook when the receiver accepts a normal HTTP request with JSON.

In Citadel:

- Type: `Generic`
- Channel URL:

```text
generic://<host>/<path>?template=json
```

Example:

```text
generic://hooks.example.com/citadel/alerts?template=json
```

## Supported Channel Types

These are the channel URL patterns supported by the Alert Rules page:

| Destination | URL format |
| --- | --- |
| Bark | `bark://<device-key>@<host>` |
| Discord | `discord://<webhook-token>@<webhook-id>` |
| Generic | `generic://<host>/<path>?template=json` |
| Gotify | `gotify://<gotify-host>/<token>` |
| Google Chat | `googlechat://chat.googleapis.com/v1/spaces/<space>/messages?key=<key>&token=<token>` |
| IFTTT | `ifttt://<key>/?events=<event>&value1=<value>` |
| Join | `join://citadel:<api-key>@join/?devices=<device>` |
| Lark | `lark://<host>/<token>?secret=<secret>` |
| Mattermost | `mattermost://<username>@<mattermost-host>/<token>/<channel>` |
| Matrix | `matrix://<username>:<password>@<host>:<port>/?rooms=<room>` |
| ntfy | `ntfy://:<access-token>@<host>/<topic>` |
| OpsGenie | `opsgenie://<host>/<token>?responders=<responder>` |
| Pushbullet | `pushbullet://<api-token>/<device-or-channel>` |
| Pushover | `pushover://citadel:<api-token>@<user-key>/?devices=<device>` |
| Rocket.Chat | `rocketchat://<username>@<rocketchat-host>/<token>/<channel>` |
| Signal | `signal://<host>/<source-phone>/<recipient>` |
| Slack | `slack://<botname>@<token-a>/<token-b>/<token-c>` |
| Teams | `teams://<group>@<tenant>/<alt-id>/<group-owner>/<extra-id>?host=<webhook-host>` |
| Telegram | `telegram://<bot-token>@telegram?chats=<chat-id>` |
| WeCom | `wecom://<key>` |
| Zulip Chat | `zulip://<bot-email>:<bot-key>@<zulip-domain>/?stream=<stream>&topic=<topic>` |

## Configure A Seeded System Rule

Community administrators can configure a rule installed by Citadel:

1. Open `Monitoring -> Alert Rules`.
2. Select a seeded system rule.
3. Enable or disable the rule.
4. Select one or more notification channels.
5. Save the rule.

The rule continues creating in-app alert events when it has no channel.
Selecting an active channel also enables external delivery.

Changing severity, cooldown, thresholds, required matches, quiet hours, or
resource scope requires Team's Advanced Alerting capability.

## Create A Custom Alert Rule

Creating a custom alert rule requires Team's `Advanced Alerting` capability.

Select:

```text
Monitoring -> Alert Rules -> Add Rule
```

Set:

- `Alert Type`: condition that produces the alert. This is fixed after the rule is created.
- `Name`: stable name shown in alert events and notification titles.
- `Status`: enabled rules can trigger; disabled rules are ignored.
- `Severity`: default severity for alert events created by this rule.
- `Cooldown`: minimum time before the same rule can trigger again for the same resource.
- `Applies to`: optional resource scope. Leave empty to apply to all matching resources.
- `Notification Channels`: external destinations that should receive notifications.
- `Quiet Hours`: daily or weekly windows where matching alerts are suppressed.

Alert events are still recorded in Citadel even when no notification channel is selected. External notifications are only sent when the rule has at least one active channel.

## Threshold Rules

`PlatformCpuHigh`, `PlatformRamHigh`, and `PlatformDiskHigh` are threshold rules.

They require:

- `Threshold`: percentage that must be exceeded.
- `Required Matches`: number of consecutive checks required before the alert fires.

Use required matches to reduce noise. For example, CPU above `90` with `Required Matches` set to `3` only fires after three consecutive high CPU checks.

## Quiet Hours

Quiet hours suppress alerts during planned maintenance or noisy time windows.

Each quiet hour has:

- `Daily` or `Weekly` schedule
- start time
- end time
- timezone
- optional description

Quiet hours are evaluated using the selected timezone. Overlapping quiet-hour windows are not allowed.

## Alert Types

Platform alerts:

- `PlatformCpuHigh`
- `PlatformRamHigh`
- `PlatformDiskHigh`
- `PlatformUnreachable`
- `PlatformVersionMismatch`
- `UnmanagedContainerCreated`

Deployment alerts:

- `DeploymentImageUpdateAvailable`
- `DeploymentAutoUpdated`
- `DeploymentAutoDeployFailed`
- `DeploymentConfigurationResolutionFailed`

Stack alerts:

- `StackImageUpdateAvailable`
- `StackAutoUpdated`
- `StackAutoDeployFailed`
- `StackServiceAutoUpdated`
- `StackServiceAutoDeployFailed`
- `StackDriftDetected`
- `StackDriftAutoReconciled`
- `StackGitUpdateAvailable`
- `StackGitAutoUpdated`
- `StackGitAutoDeployFailed`
- `StackConfigurationResolutionFailed`

Webhook alerts:

- `WebhookAuthenticationFailed`
- `WebhookDispatchFailed`
- `WebhookGitRepoSyncFailed`
- `WebhookStackGitDeployFailed`

Automation alerts:

- `AutomationActionRunFailed`

## Recommended Rules

Creating the custom rules in this section requires Team's Advanced Alerting
capability. Community administrators can instead attach notification channels
to the corresponding seeded system rules.

For platform availability:

- Type: `PlatformUnreachable`
- Severity: `Critical`
- Cooldown: `300` to `900` seconds
- Scope: production platforms
- Channels: operations Discord, Teams, or incident channel

For sustained resource pressure:

- Type: `PlatformCpuHigh` or `PlatformRamHigh`
- Severity: `Warning`
- Threshold: `85` to `95`
- Required Matches: `3` or higher
- Cooldown: `900` seconds or higher

For failed automation:

- Type: `AutomationActionRunFailed`
- Severity: `Warning` or `Critical`
- Scope: important actions
- Channels: the team that owns the action

For deployment or stack failures:

- Type: `DeploymentAutoDeployFailed`, `StackAutoDeployFailed`, or `StackGitAutoDeployFailed`
- Severity: `Critical`
- Scope: production resources
- Channels: deployment owners

## Troubleshooting

If the test notification fails:

- Check that the channel URL matches the format for the selected channel type.
- Check that the Citadel server has outbound network access to the destination service.
- URL-encode special characters in tokens, webhook IDs, or path values when required.
- Confirm the webhook, bot token, or destination channel still exists.
- Confirm the channel is enabled.

If Citadel records alert events but sends no notification:

- Make sure the rule has at least one notification channel selected.
- Make sure the selected channel is active.
- Check whether the rule is inside quiet hours.
- Check whether the cooldown has not elapsed yet.
- Review Citadel server logs for notification delivery errors.

If a rule does not trigger:

- Make sure the rule is enabled.
- Check that the selected resources in `Applies to` include the resource you expect.
- For CPU, RAM, and disk rules, confirm the threshold and required match count are reachable.
- For stack, deployment, webhook, and automation rules, confirm the underlying feature is enabled and producing events.

If alerts are too noisy:

- Increase cooldown.
- Increase required matches for CPU, RAM, and disk rules.
- Limit the rule to specific resources.
- Add quiet hours for maintenance windows.
