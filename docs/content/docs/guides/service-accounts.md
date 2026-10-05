---
title: "Service Accounts"
description: "Create scoped machine identities and tokens for integrations and automation."
---

Service Accounts give CI/CD runners, deployment pipelines, and other external
services a non-human Citadel identity.

Use a Service Account when software needs to call the Citadel API or when an
unattended Citadel operation should not depend on one employee's account.

Do not create a normal User with a shared email address and password for this
purpose. Service Accounts cannot sign in to the Citadel Web UI and do not have
passwords, MFA, OIDC login, refresh tokens, or browser sessions.

## Availability

Service Accounts require Team's **Custom access control** capability. An Enterprise
or design-partner license can also enable them when that capability is included
in its signed capability list.

The capability enables account creation, API tokens, token
authentication, and selecting an account as a run-as identity. Citadel does not
license the number of accounts or tokens; token lifetime and active-token
limits are security safeguards.

The permissions assigned to an account still follow normal licensing:

- built-in and custom roles and resource-specific access require Custom Access Control;
- schedules, webhooks, and other unattended operations still require the
  relevant Automated Operations capability;
- every API operation enforces its own Citadel permission and feature rules.

If the capability becomes unavailable, Citadel keeps the accounts, access
assignments, run-as bindings, token metadata, and history. Authentication and
new run-as execution are suspended. Administrators can still inspect accounts,
disable them, revoke tokens, and archive unused accounts. Restoring an eligible
license reactivates unrevoked tokens that are either unexpired or non-expiring
for enabled accounts.

## Open Service Accounts

Open:

```text
Settings > Access > Service Accounts
```

The table shows each account's status, direct roles, Team memberships, active
token count, and most recent token use.

Without the required capability, the tab displays the Team license indicator.
Existing accounts remain visible after a downgrade, but creation, token
issuance, enabling, access expansion, and new run-as bindings are unavailable.

Only administrators can create an account, change its access, enable or disable
it, or archive it.

## Create A Service Account

Select **Add Service Account**, then configure:

- **Name**: a stable name that identifies the integration, such as
  `production-release`;
- **Description**: who owns the integration and what it is allowed to do;
- **Enabled**: whether its tokens and run-as operations can currently be used;
- **Teams**: Team permissions inherited by the account;
- **Roles**: roles assigned directly to the account;
- **Resource access**: additional permission for selected Citadel resources.

Creating an account does not automatically create a token. Save the account
first, then create a token only when an integration needs one.

Roles, Teams, and resource access are empty by default. A Service Account never
inherits the administrator's permissions merely because that administrator
created it.

## Assign Least-Privilege Access

A Service Account receives permissions from the same sources as a User:

- directly assigned roles;
- enabled Teams containing the account;
- resource access assigned directly to the account;
- resource access inherited through an enabled Team.

These permissions are additive. A resource access entry grants additional
access; it is not a deny rule.

Prefer one account per trust boundary. For example, use separate accounts for a
production release pipeline and a reporting integration. Avoid one shared
administrator account for every pipeline.

Create a purpose-specific custom role containing only the operations the
integration needs, then restrict it to the relevant resources where possible.
A generic machine role is usually unsafe because a deployment pipeline, backup
worker, and reporting integration need different access.

Avoid assigning the built-in **Admin** role. It gives every active and future
token unrestricted Citadel authority, so disclosure of one token can compromise
the installation. Citadel displays a high-severity confirmation before Admin is
assigned and keeps a **Full administrative access** warning visible on the
account and credential screens while it remains assigned. Use Admin only for an
exceptional, explicitly reviewed integration.

### Use

The **Use** capability lets a User select a Service Account as the execution
identity for an Automation Action, Backup Policy, or another supported run-as
resource.

It does not create a token and does not let the User make arbitrary requests as
the Service Account.

### Manage Credentials

The **Manage Credentials** capability lets a User create and revoke tokens for
a visible Service Account. Because a token can exercise the account's current
permissions, grant this capability sparingly.

Service Accounts cannot create accounts or manage Service Account credentials,
even if one of their roles contains those permissions.

## Create A Token

Open the saved Service Account, find **Credentials**, and select
**Create token**.

Enter:

- a unique token name describing its consumer, such as
  `github-actions-2026-11`;
- an expiration of 30, 90, 180, or 365 days, **Never**, or a permitted custom
  date.

The default lifetime is 90 days and the maximum finite lifetime is 365 days.
One account can have up to 10 active tokens unless the installation policy
changes those limits.

Choose **Never** only when the integration cannot rotate credentials. Citadel
shows a high-severity confirmation because the bearer token remains valid until
you revoke it, disable or archive the account, or the Custom access control license
capability becomes unavailable. The credentials table displays `Never` for its
expiration. Prefer a finite lifetime whenever practical.

After creation, Citadel displays the plaintext token once:

> Copy this token now. Citadel cannot display it again.

Copy it into the CI/CD platform's protected secret store before closing the
dialog. Citadel stores a digest and cannot recover the original value.

If the token is lost, revoke it and create a replacement.

## Call The Citadel API

Send the token through the HTTP `Authorization` header:

```bash
curl \
  -H "Authorization: Bearer <service-account-token>" \
  https://citadel.example.com/api/v1/platforms/
```

Use HTTPS for remote Citadel installations. Never place a token in a URL,
query string, source repository, script output, or ordinary configuration
file.

Citadel checks the Service Account's current roles, Team memberships, resource
access, enabled state, and Custom access control license capability on every request.
Permission changes take effect without creating a new token.

A valid token receives the same forbidden or hidden-resource behavior as a
User without permission. It does not bypass Citadel authorization.

## Rotate A Token

Use overlapping tokens to rotate without interrupting an integration:

1. Create a second token with a new descriptive name.
2. Store it in the external service's protected secret store.
3. Update and verify the external service.
4. Revoke the old token.

Revocation is permanent. Other active tokens belonging to the same account
continue working.

Token names remain in credential history and cannot be reused. Include the
consumer or rotation date in the name.

## Disable, Revoke, Or Archive

These actions have different effects:

| Action | Effect | Reversible |
| --- | --- | --- |
| Disable account | Suspends every token and prevents new run-as execution. | Yes |
| Revoke token | Permanently invalidates one token. | No |
| Archive account | Disables the account and permanently revokes all tokens. | No |
| License unavailable | Suspends Service Account authentication and run-as use without changing token state. | Yes, by restoring the capability |

Re-enabling an account restores only unrevoked tokens that have not expired or
were explicitly created with **Never**.

Disabling an account that is used by active scheduled resources is allowed.
The confirmation identifies those active consumers because their future runs
will fail until the account is re-enabled or replaced.

Archiving can be blocked when an active Automation Action, Backup Policy, or
another resource still uses the account. Replace its **Run As** selection or
disable/archive the dependent resource first. Historical runs remain readable.

## Use With Automation Actions

Select an enabled Service Account in an Action's **Run As** field when the
Action should use stable non-human permissions.

This requires the **Custom access control** capability. Scheduled and
webhook-triggered Actions also require **Automated Operations**.

Citadel distinguishes:

- **Triggered by**: the User or Service Account that requested the run;
- **Ran as**: the configured identity whose permissions the Action used.

A CI/CD Service Account may trigger an existing Action when it has Execute
permission on that Action. The Action still runs as its configured identity;
the caller cannot replace the code or run-as account in the run request.

Citadel creates a short-lived internal token for each Action run. It does not
copy or expose the Service Account's persistent API token to the script.

See [Automation Actions](/docs/resources/automation-actions) for Action configuration and
execution behavior.

## Use With Backup Policies

Select a Service Account as the Backup Policy's **Run As** identity for
scheduled and webhook-triggered backups. Assign only the permissions needed to
read the protected source and use the configured repository.

This requires the **Custom access control** capability. Scheduled and
webhook-triggered backups also require **Automated Operations**.

Manual backups use the authenticated caller. Backups requested from an
Automation Action use that Action run's identity.

See [Backups](/docs/resources/backups) for source, repository, scheduling, and restore
behavior.

## Service Account Tokens And Webhook Secrets

They are different credentials:

- a **Service Account token** authenticates a Citadel Actor and receives its
  current RBAC and resource access;
- a **Generic / CI webhook secret** authenticates delivery to one configured
  webhook listener and does not receive Citadel permissions.

Do not paste a Service Account token into a webhook secret field. Use a Service
Account token when calling an authenticated `/api/v1` endpoint, and use the
generated webhook secret only with that resource's listener URL.

See [Webhooks](/docs/guides/webhooks) for provider-specific listener authentication.

## Troubleshooting

### The API Returns Unauthorized

Check that:

- the complete token was copied;
- the token has not expired or been revoked, or was explicitly created with
  **Never**;
- the Service Account is enabled and not archived;
- the installed license includes the Custom access control capability and is active
  or in its grace period;
- the request uses the `Authorization: Bearer` header;
- the token was not placed in a query string;
- the request is being sent to the correct Citadel installation.

Citadel intentionally does not reveal which token validation check failed.

### The API Returns Forbidden Or Not Found

The token is valid, but the Service Account cannot perform or see the requested
operation. Review its direct roles, Team memberships, resource access, and the
license capability required by the endpoint.

### A Token Cannot Be Revealed

This is expected. Citadel displays plaintext only after token creation. Revoke
the lost token and create a replacement.

### An Account Cannot Be Archived

Review **Used by** on the Service Account. Replace or disable each active
run-as reference, then archive the account again.


