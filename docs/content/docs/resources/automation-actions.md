---
title: "Automation actions"
description: "Create, authorize, run, and troubleshoot Citadel automation actions."
---

Automation Actions let you run small TypeScript scripts inside Citadel.

Use actions for operational tasks that you want to run from Citadel, schedule, or trigger from another system.

Examples:

- apply a stack
- run a maintenance check
- call Citadel APIs in a repeatable way
- accept a webhook payload and dispatch a Citadel operation

Actions run through Citadel permissions. They are not raw shell scripts and they do not get host or Docker access.

## Run a prepared action

This example checks access to one Stack, then lets you explicitly enable its
deployment. Ask the application's maintainer to review scripts you do not maintain.

### 1. Prepare the Action and identity

1. Open **Automation** and select **Add Action**.
2. Enter a name and choose an enabled **Run as** User or Service Account under
   **Execution → Runtime**. The initial check needs Read access to the selected
   Stack; deploying it also requires Execute and Apply. See [access control](/docs/guides/access-control).
3. Leave the Action, schedule, and webhook disabled while preparing the example.
4. Set **Default Args** to the following object, replacing the ID with your
   Stack's ID:

```json
{
  "stackId": "019f0000-0000-7000-9000-000000000000",
  "dryRun": true
}
```

Selecting a Service Account requires **Use** permission on that account and
Team's **Custom access control** capability. The person testing or running the
Action also needs Execute access to the Action itself.

### 2. Add the script and test

Paste this into **Code**, save the Action, then select **Test**:

```ts
const stackId = args.stackId;
if (typeof stackId !== "string" || !stackId) {
  throw new Error("Set stackId to the Stack's ID in Default Args.");
}

await citadel.api.getStack(stackId);

if (args.dryRun !== false) {
  console.log("Access check passed; no deployment requested.");
} else {
  await citadel.api.applyStack({ id: stackId });
  console.log("Apply returned successfully; check the Stack's operation and health.");
}
```

Keep the progress sheet connected until completion. Confirm **Succeeded** and
the access-check message in **Runs**. This checks Read access only; it does not
prove that the identity can deploy the Stack.

`dryRun` is a convention implemented by this script. **Test** itself can perform
real operations, including with unsaved editor changes. Citadel does not simulate
or undo an Action's API calls.

### 3. Run the operation

Set `dryRun` to `false` only when you intend to deploy the selected Stack. Enable
the Action, save, then select **Run**. Check the Action's result and the Stack's
operation history, running services, and application endpoint.

After verifying the manual operation, configure a [schedule](#schedule) or
[webhook](#webhook) if needed. These triggers require **Automated Operations**.
Save reviewed code and arguments before enabling them; webhook payloads must
supply the fields the script expects.

## Open Actions

Open **Automation** to see each Action's enabled state, triggers, latest run,
and available operations. Open an Action for its configuration and run history.

## Create An Action

Select:

```text
Automation -> Add Action
```

Set:

- `Name`: stable action name shown in activity and run history
- `Description`: optional note for other admins
- `Enabled`: whether manual and scheduled runs are allowed
- `Code`: TypeScript code executed by Deno
- `Default Args`: JSON object available as `args`
- `Run As`: enabled User or Service Account whose current Citadel permissions
  are used for runs
- `Timeout`: maximum run duration in seconds
- `Alert On Failure`: record failed or timed-out runs as alert-worthy events

Save the action before using manual, scheduled, or webhook triggers.

## Script Globals

Action code can use these globals:

```ts
citadel
args
run
console
```

`args` is the JSON object configured on the action or passed by a trigger.

`run` contains run metadata:

```ts
console.log(run.id);
console.log(run.actionName);
console.log(run.trigger);
```

`citadel` is a preconfigured helper for Citadel API calls. You do not need to add a base URL or token.

```ts
const deployments = await citadel.get("/api/v1/deployments");
console.log(deployments);
```

The code editor knows the built-in globals and uses Citadel's generated API types for completions on `citadel`, `args`, and `run`.

Use the generated operation names through `citadel.api`:

```ts
const platforms = await citadel.api.listPlatforms();
const repositories = await citadel.api.listGitRepositories();
console.log(platforms, repositories);
```

For deployment and stack operations, replace the placeholder IDs below with
real resource IDs. Rollback also needs the ID of a previous healthy release:

```ts
await citadel.api.applyDeployment({
  id: "<deployment-id>"
});

await citadel.api.applyStack({
  id: "<stack-id>"
});

await citadel.api.rollbackStack({
  stackId: "<stack-id>",
  releaseId: "<healthy-release-id>"
});
```

These are separate examples; include only the operation your Action needs.
The request body must match the Citadel API endpoint. The current runtime
does not expose the `apply` and `rollback` shortcuts or the `repositories`
alias suggested by some editor completions; use the operation names above.

## Arguments

**Default Args** must be a JSON object, not an array or a plain string. Validate
required fields before calling an API, as in the [prepared Action](#run-a-prepared-action).
Use an explicit check such as `args.dryRun !== false` when an absent field should
leave an operation disabled.

Path parameters are positional in the runtime helper, for example
`citadel.api.getStack(stackId)`. Request bodies are objects, for example
`citadel.api.applyStack({ id: stackId })`. Use the endpoint's request shape;
an accepted request may queue work that must be checked separately.

## Test Run

Use `Test` on the action edit page to queue a test run.

Test runs:

- create an action run with trigger `Test`
- can run even when the action is disabled
- use the current editor code and **Default Args**, including unsaved changes
- use the saved **Run as** identity; save identity changes before testing
- write logs to the Runs tab

Testing does not save draft code. Save the reviewed version before a normal Run,
schedule, or webhook should use it. A disabled Action can still be tested, so
disabling it is not a way to make tests read-only.

## Manual Run

Use `Run` to queue a normal manual run.

Manual runs require the action to be enabled.

If the same action is already running, Citadel rejects the new run instead of running the same action twice.

Manual and test runs show live output in the progress sheet. Keep the connection
open until the run finishes; disconnecting from an interactive run requests
cancellation. Cancellation does not undo operations the script has already
completed. Check the Runs tab before retrying.

## Schedule

Schedules require Team's `Automated Operations` capability.

Save and enable the Action, then enable **Schedule** to run it automatically.
The schedule uses saved code, arguments, and the configured execution identity.

Set:

- `Enabled`
- `Cron`
- `Time Zone`

Example cron values:

```text
*/15 * * * *     every 15 minutes
0 * * * *        hourly
0 2 * * *        daily at 02:00
0 2 * * 1        Mondays at 02:00
```

[![Automation runtime and schedule with an Operations runner identity, a 120 second timeout, failure alerts, and a weekday UTC schedule](/screenshots/automation-runtime-schedule.png)](/screenshots/automation-runtime-schedule.png)

This demo Action uses the `Operations runner` identity, a 120-second timeout,
and `0 8 * * 1-5` to run at 08:00 UTC on weekdays. Review **Run as** and its
permissions together with the schedule before enabling unattended execution.

When a scheduled run is due, Citadel queues an action run with trigger `Schedule`.

If the action is already running, Citadel records a rejected run instead of starting a duplicate.

## Webhook

Webhook-triggered execution requires Team's `Automated Operations` capability.

Enable `Webhook` to let a Git webhook queue the action.

For the shared listener model, authentication options, URL shape, and troubleshooting, see [Webhooks](/docs/guides/webhooks).

Select the provider and authentication format, set an optional branch filter, and copy the listener URL into the Git provider webhook settings.

Webhook URL shape:

```text
https://citadel.example.com/listener/github/automation-action/<action-id>/run
```

For GitLab, use `/listener/gitlab/automation-action/<action-id>/run`.

If the request body is a JSON object, Citadel passes it to the action as `args`. Push event metadata remains available in the payload, and the branch filter applies to the `ref` branch.

Example webhook body:

```json
{
  "ref": "refs/heads/main",
  "after": "9fceb02...",
  "repository": {
    "full_name": "acme/app"
  }
}
```

Action code:

```ts
const branch = args.ref?.replace("refs/heads/", "");
console.log(`Triggered by ${args.repository?.full_name} on ${branch}`);
```

## Runs Tab

The Runs tab shows recent action runs.

Columns include:

- status
- trigger
- queued time
- duration
- exit code

Select the log button on a run to inspect logs.

Run statuses:

- `Queued`: waiting for a worker
- `Running`: Deno process is active
- `Succeeded`: process exited successfully
- `Failed`: process exited with an error
- `TimedOut`: timeout was reached
- `Cancelled`: cancellation was requested
- `Rejected`: Citadel refused to start the run

Logs include output from:

```ts
console.log()
console.error()
```

Citadel caps logs and redacts obvious sensitive values such as bearer tokens and password-like fields.

## Activities Tab

The Activities tab shows audit history for the action.

Activity events include:

- created
- updated
- renamed
- deleted
- enabled or disabled
- run queued
- run started
- run succeeded
- run failed
- run timed out
- run cancelled
- run rejected

Updated action events show a Monaco diff of the old and new action values.

## Permissions

Actions use the `AutomationAction` permission resource.

Callers need permission to:

- view actions
- create actions
- update actions
- delete actions
- run or test actions

An Action runs as its configured **Run as** User or Service Account. Citadel
checks that identity's current Roles, enabled Team memberships, resource access,
and license capabilities when the Action calls Citadel APIs.

| Identity | What it controls |
| --- | --- |
| Person or integration starting a run | Must have Execute permission on the Action |
| Saved **Run as** identity | Supplies access to the resources and operations called by the script |
| Editor selecting a Service Account | Needs **Use** permission on that account |

An administrator clicking Run does not make the script administrative when its
Run-as identity has narrower permissions. Conversely, Execute access to an
Action allows a caller to invoke its saved operation under that identity.

Manual and API-triggered run history distinguishes the identity that requested
the run from the identity that executed it. For example:

```text
Triggered by ci-release
Ran as production-deployer
```

Selecting a Service Account or changing executable Action configuration under
that account requires permission to **Use** it. Running an already saved Action
requires Execute permission on the Action. Normal run requests cannot replace
its saved code or run-as identity. Test requests can supply draft code; see
[Test Run](#test-run).

Using a Service Account requires Team's **Custom access control** capability.
Schedules and webhook-triggered execution under that account additionally
require **Automated Operations**. If the Custom access control capability becomes
unavailable, Citadel preserves the binding as `Paused by license` and rejects
new runs before starting the script.

Citadel does not store a User access/refresh token or a persistent Service
Account token inside the Action. It creates a restricted, short-lived token for
each run.

Use a least-privileged Service Account for schedules and webhooks so unattended
execution does not depend on a human account. See
[Service Accounts](/docs/guides/service-accounts).

## Safety Notes

Actions are TypeScript scripts, not shell scripts.

Actions cannot:

- run shell commands
- use terminal or exec endpoints
- access arbitrary host files
- access arbitrary networks
- read Citadel secrets directly
- modify automation endpoints through the automation run token

Use actions for Citadel API automation, not host administration.

## Troubleshooting

`Action is already in progress.`

The same action already has a queued or running run. Wait for it to finish or cancel the active run.

`Automations are disabled.`

An administrator disabled automation globally in Citadel configuration.

`Deno executable was not found.`

The automation runtime is unavailable. Contact a Citadel administrator.

`Default arguments must be a valid JSON object.`

The args editor must contain an object such as `{}` or `{ "stackId": "..." }`, not an array or plain string.

`Action timed out.`

The script ran longer than the configured timeout. Increase the timeout or change the script to complete faster.

`Forbidden` from a Citadel API call.

The configured run-as User or Service Account does not currently have
permission for the operation the script attempted.

`Run-as identity is disabled or unavailable.`

Enable the configured identity or select another enabled User or Service
Account. Citadel does not fall back to the Action creator or System.

## License Availability

Community can:

- create and edit action definitions
- test an action
- start an enabled action manually
- view runs and logs

Schedules and webhook-triggered action execution require Team's
`Automated Operations` capability. A webhook may be received and authenticated
without that capability, but it cannot queue the action run.

If the capability becomes unavailable, existing schedule and webhook
configuration remains stored and is shown as paused. Manual and test runs remain
available.
