---
title: "Alert rules"
description: "Configure alert rules, delivery channels, evaluation, and recovery behavior."
---

Alert rules record operational events and notify the people responsible for the
affected resources. Start with the built-in rules, then tune them when you know
which conditions need attention.

Notifications are delivered through notification channels. The Citadel server image includes notification delivery support, so normal Docker installs do not need any extra notification service.

## Receive your first notification

1. Open **Settings → Alert Rules**.
2. Add a **Notification Channel** for your preferred destination.
3. Use **Send Test Notification**, confirm it arrives, then **Save** the channel.
4. Assign the channel to a suitable built-in rule, such as **Platform Unreachable**,
   enable the rule, and save it.
5. When a matching condition occurs, find its event in **Alerts** and confirm
   the message arrives at the destination.

A channel test checks delivery using the dialog's values. It does not exercise
the rule's scope, thresholds, or cooldown. Verify those separately; there is no
need to interrupt a production Platform to test a channel.

Community includes the built-in system rules and notification channels. Creating
custom rules and changing advanced rule behavior require Advanced Alerting.
The provider setup below explains where to obtain each destination URL. See
[License Availability](#license-availability) for the full feature breakdown.

## Respond to an alert

1. Open **Alerts**, leave **Unresolved only** enabled, and select the alert type
   in a row to open its details.
2. Read the affected resource, recorded time, and **Info**. Follow the resource
   link and inspect its current state, operation history, and relevant logs.
3. Select **Acknowledge** while investigating. The event remains unresolved.
4. Correct the cause, then verify recovery on the resource: fresh Platform
   readings, a successful operation, or a working application endpoint.
5. Select **Resolve** if the event remains open after recovery. Turn off
   **Unresolved only** to find resolved events later.

| Status | Meaning |
| --- | --- |
| Active | The event is open and has not been acknowledged. |
| Acknowledged | Someone has acknowledged it; investigation or recovery may still be needed. |
| Resolved | The event was closed manually or by a recovery observation. |

Acknowledge and Resolve update the alert record; they do not restart workloads,
retry failed jobs, or change a rule. For monitored conditions such as Platform
availability, resource pressure, and Build Pool availability, Citadel can resolve
the incident when it observes recovery. A historical failed operation can still
need manual resolution after a successful retry.

Use [Platform monitoring](/docs/operations/platform-monitoring) to investigate
capacity alerts and [Troubleshooting](/docs/operations/troubleshooting) for
connection or application failures. Alert details describe the condition; they
are not a per-channel delivery receipt.

## Before You Start

Channel URLs often contain webhook IDs, tokens, or bot credentials. Treat them as secrets.

Use a separate webhook or bot token for Citadel when the destination service supports it. If the URL is exposed later, you can revoke only the Citadel webhook or token without breaking other integrations.

## Notification Channels

1. Open **Settings → Alert Rules** and select **Add Channel** in the
   **Notification Channels** section.
2. Enter a descriptive **Name**, such as `Operations email`.
3. Select the **Type** and enter the **Channel URL** using the matching setup below.
4. Leave **Is Active** enabled to allow alert delivery.
5. Select **Send Test Notification** and check the destination for the message.
6. Select **Save**. Testing uses the values in the dialog; it does not save the channel.
7. Open a rule, select the channel under **Notification Channels**, enable the
   rule, and save it.

A saved channel does not receive alerts until it is assigned to an enabled rule.
You can reuse a channel across rules or select several channels for one rule.
Notifications are sent by the Citadel server, so the destination must be reachable
from the server's container.

## Channel URL Format

The channel URL tells Citadel which service to use and which token, webhook, topic, or chat to send to.

Most URLs follow this shape:

```text
service://credentials@destination/path?options
```

Replace placeholder values such as `<token>`, `<webhook-id>`, and `<chat-id>` with the values from the destination service. If a token contains special URL characters such as `@`, `/`, `?`, `&`, or `#`, URL-encode that value before saving it.

## Configure Common Channels

Citadel uses [Shoutrrr](https://github.com/nicholas-fedor/shoutrrr) service URLs.
Select the matching **Type** in Citadel, then enter the URL described below.
The screenshots show Citadel's actual forms with demonstration values; replace
all example credentials and destinations with your own.

### Email (SMTP)

Select **Email** to deliver notifications through an SMTP server.

1. Obtain your provider's SMTP hostname, port, credentials, and an authorized
   sender address. Use an application password if your provider requires one.
2. Enter the sender in `from` and the recipient in `to`.
3. For port **587** with required STARTTLS, use:

```text
smtp://<username>:<password>@<smtp-host>:587/?from=alerts@example.com&to=ops@example.com&requirestarttls=yes
```

For port **465** with implicit TLS, use:

```text
smtp://<username>:<password>@<smtp-host>:465/?from=alerts@example.com&to=ops@example.com&encryption=ImplicitTLS
```

Use the port and authentication method specified by your provider. Add
`&auth=Login` if it requires LOGIN authentication. Separate multiple recipients
with commas: `to=ops@example.com,oncall@example.com`.

URL-encode credentials before inserting them: a username of `alerts@example.com`
becomes `alerts%40example.com`, and a password containing `#` uses `%23`.

See the [SMTP reference](https://github.com/nicholas-fedor/shoutrrr/blob/v0.19.0/docs/services/email/smtp/index.md)
for additional authentication and sender options.

[![Citadel notification channel dialog with Email selected and an example SMTP URL](/screenshots/notification-channel-email.png)](/screenshots/notification-channel-email.png)

Select **Send Test Notification**, check the recipient's inbox and spam folder,
then **Save** and assign the channel to a rule.

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

The token goes before `@`; the numeric webhook ID goes after it. Keep the webhook
in a channel intended for operational notifications.

[![Citadel notification channel dialog with Discord selected and a demonstration webhook URL](/screenshots/notification-channel-discord.png)](/screenshots/notification-channel-discord.png)

Select **Send Test Notification** and confirm the message appears in the chosen
Discord channel. Then **Save** and assign it to a rule.

### Microsoft Teams

1. Create a Power Automate workflow with the **When a Teams webhook request is
   received** trigger and an action that posts to the desired Teams channel.
2. Copy the generated workflow webhook URL.
3. Percent-encode the **entire** URL, including its query string, and use it as
   the `host` parameter below.
4. Select **Teams** in Citadel.

```text
teams://?host=<percent-encoded-workflow-url>
```

For example, `https://` becomes `https%3A%2F%2F`, `?` becomes `%3F`, and `&`
becomes `%26`. Do not paste the unencoded workflow URL after `host=`: its query
parameters would be interpreted as notification options.

The bundled sender uses workflow webhooks. Replace existing legacy
`teams://group@tenant/...` configurations with this format. See the
[Teams reference](https://github.com/nicholas-fedor/shoutrrr/blob/v0.19.0/docs/services/chat/teams/index.md)
for a complete conversion example.

Test the channel, confirm the workflow posts the message, then save and assign it.

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

Keep the three path segments in the same order, and omit `/services/`.
Test the channel and confirm the message arrives before saving and assigning it.

You can include a bot name:

```text
slack://citadel@<token-a>/<token-b>/<token-c>
```

### Telegram

In Telegram:

1. Create a bot with BotFather.
2. Copy the bot token.
3. Add the bot to the target chat, group, or channel.
4. Use a numeric chat ID for a private chat or group, or an `@username` for a
   public channel. An invite link is not a chat ID.

For help finding IDs, follow the
[Telegram setup reference](https://github.com/nicholas-fedor/shoutrrr/blob/v0.19.0/docs/services/chat/telegram/index.md).
Give the bot permission to post in the destination.

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

1. Choose your ntfy server and topic.
2. Subscribe to that same server and topic in your ntfy app or web client.
3. Select **Ntfy** in Citadel and configure its publishing URL.

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
2. Copy the **application** token for sending messages.
3. Open a Gotify client connected to the same server so you can verify delivery.

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

The JSON template sends `title` and `message` fields. Confirm the receiver accepts
this payload; an arbitrary webhook may require different fields or headers. See
[Generic webhook options](https://github.com/nicholas-fedor/shoutrrr/blob/v0.19.0/docs/services/specialized/generic/index.md)
for customization.

## Supported Channel Types

These are the channel URL patterns supported by the Alert Rules page:

| Destination | URL format |
| --- | --- |
| Bark | `bark://<device-key>@<host>` |
| Discord | `discord://<webhook-token>@<webhook-id>` |
| Email | `smtp://<username>:<password>@<host>:587/?from=<sender>&to=<recipient>&requirestarttls=yes` |
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
| Teams | `teams://?host=<percent-encoded-workflow-url>` |
| Telegram | `telegram://<bot-token>@telegram?chats=<chat-id>` |
| WeCom | `wecom://<key>` |
| Zulip Chat | `zulip://<bot-email>:<bot-key>@<zulip-domain>/?stream=<stream>&topic=<topic>` |

## Configure A Seeded System Rule

Community administrators can configure a rule installed by Citadel:

1. Open `Settings -> Alert Rules`.
2. Select a seeded system rule.
3. Enable or disable the rule.
4. Select one or more notification channels.
5. Save the rule.

An enabled rule continues creating in-app alert events when it has no channel.
Selecting an active channel also enables external delivery.

Changing severity, cooldown, thresholds, required matches, quiet hours, or
resource scope requires Team's Advanced Alerting capability.

## Create A Custom Alert Rule

Creating a custom alert rule requires Team's `Advanced Alerting` capability.

Select:

```text
Settings -> Alert Rules -> Add Rule
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

- `Threshold`: percentage that must be reached or exceeded.
- `Required Matches`: number of consecutive checks required before the alert fires.

For example, a threshold of `90` with **Required Matches** set to `3` requires
three consecutive evaluated readings at or above 90%. CPU and RAM evaluations
use smoothed samples; the count is not a duration in seconds or a count of
browser chart updates. A reading below the threshold resets the count.

### Overlapping rules and cooldown

For the same alert type and resource, Citadel selects the highest-severity
matching rule. Only that rule can trigger for that observation; a rule still
waiting for required matches or cooldown does not fall back to a lower-severity
match. Attach the intended channels to each severity you use.

An existing open incident prevents duplicate events for the same condition.
**Cooldown** limits how soon a new event can be created for the rule and resource;
it is not a reminder interval for an unresolved alert.

## Quiet Hours

Quiet hours skip rule evaluation during planned maintenance or noisy time windows.
They suppress new in-app events as well as external notifications. Existing
events remain visible, and automatic recovery is evaluated after quiet hours
end when a new observation arrives.

Each quiet hour has:

- `Daily` or `Weekly` schedule
- start time
- end time
- timezone
- optional description

Quiet hours are evaluated using the selected timezone. Overlapping quiet-hour windows are not allowed.

## Alert Types

Choose a type based on the condition you need to investigate:

| Area | Alert types |
| --- | --- |
| Platform capacity | `PlatformCpuHigh`, `PlatformRamHigh`, `PlatformDiskHigh` |
| Platform availability and inventory | `PlatformUnreachable`, `PlatformVersionMismatch`, `UnmanagedContainerCreated` |
| Deployment | `DeploymentImageUpdateAvailable`, `DeploymentAutoUpdated`, `DeploymentAutoDeployFailed`, `DeploymentConfigurationResolutionFailed` |
| Swarm Service | `SwarmServiceOperationFailed` |
| Stack images and services | `StackImageUpdateAvailable`, `StackAutoUpdated`, `StackAutoDeployFailed`, `StackServiceAutoUpdated`, `StackServiceAutoDeployFailed` |
| Stack drift | `StackDriftDetected`, `StackDriftAutoReconciled` |
| Stack Git and configuration | `StackGitUpdateAvailable`, `StackGitAutoUpdated`, `StackGitAutoDeployFailed`, `StackConfigurationResolutionFailed` |
| Webhook | `WebhookAuthenticationFailed`, `WebhookDispatchFailed`, `WebhookGitRepoSyncFailed`, `WebhookStackGitDeployFailed` |
| Automation | `AutomationActionRunFailed` |
| Builds | `BuildRunFailed`, `BuildAgentPoolUnavailable` |
| License | `LicenseEnteredGracePeriod`, `LicenseExpired` |

`BuildAgentPoolUnavailable` uses an **Unavailable grace period (seconds)** threshold,
rather than a percentage. See [Build Pool availability](/docs/resources/build-pools#build-pool-connection-and-availability)
for the built-in rule's grace period and recovery behavior.

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

For failed Swarm Service operations:

- Type: `SwarmServiceOperationFailed`
- Severity: `Critical`
- Scope: production Swarm Services
- Channels: service owners

## Troubleshooting

If the test notification fails:

- Check that the channel URL matches the format for the selected channel type.
- Check that the Citadel server has outbound network access to the destination service.
- URL-encode special characters in tokens, webhook IDs, or path values when required.
- Confirm the webhook, bot token, or destination channel still exists.
- For email, check the SMTP credentials, sender authorization, port, and TLS settings.
  If the test succeeds but the inbox is empty, check spam filtering and the mail provider's delivery logs.

If Citadel records alert events but sends no notification:

- Check that the rule had the intended active channel assigned when the event
  was created. Assigning a channel later does not resend existing events.
- Test the channel again and check the destination's delivery or workflow logs.
- Delivery is asynchronous and failed sends are retried, up to eight attempts.
  An event in **Alerts** does not prove that the external message arrived.
- Check Citadel server logs for delivery-worker errors if channel tests succeed
  but new matching events still produce no messages.

If a rule does not trigger:

- Make sure the rule is enabled.
- Check that the selected resources in `Applies to` include the resource you expect.
- Check quiet hours, cooldown, and whether the same condition already has an
  open incident. A higher-severity matching rule can also take precedence.
- For CPU, RAM, and disk rules, confirm fresh readings meet the threshold and
  required match count. Missing metrics are not evidence of recovery.
- For stack, deployment, webhook, and automation rules, confirm the underlying feature is enabled and producing events.
- For custom rules, confirm **Advanced Alerting** is available.

If alerts are too noisy:

- Increase cooldown.
- Increase required matches for CPU, RAM, and disk rules.
- Limit the rule to specific resources.
- Add quiet hours for maintenance windows.

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
