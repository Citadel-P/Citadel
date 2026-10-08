---
title: "Licensing"
description: "Understand Citadel source licensing, Community and Team capabilities, and product-license behavior."
---

Citadel works without an installed paid product license. An installation
without a product key runs as the Community edition under the source license.

A paid product license is needed only for paid capabilities. Citadel does not
license ordinary resource counts such as platforms, stacks, deployments,
builds, backup definitions, or automation definitions.

## Source Licensing and Product Entitlements

Unless separately identified, Citadel-authored Core, Agent, frontend and feature
implementations in this revision are offered under the
[Elastic License 2.0](https://github.com/Citadel-P/Citadel/blob/main/LICENSE)
(`Elastic-2.0`). Citadel is **source-available, not OSI-approved open source**.
ELv2 permits use, modification and redistribution subject to its conditions.
In particular, it restricts:

- providing the software to third parties as a hosted or managed service that
  gives them access to a substantial set of its features or functionality;
- moving, changing, disabling or circumventing license-key functionality, or
  removing or obscuring functionality protected by a license key;
- altering, removing or obscuring the licensor's licensing, copyright or other
  notices.

Recipients of copies must receive the license terms, and modified copies must
carry prominent modification notices. The full LICENSE controls these rights;
this summary does not replace it or introduce additional restrictions.

Community is free for personal self-hosting and internal business production
use under ELv2. A company does not need Team merely because it is a company.
Team product keys enable paid capabilities; they do not waive ELv2 or grant
hosted-service rights. The source license does not provide a signed Team key.

Consultant setup work and internal use by service providers are not the same
as selling customer access to substantial Citadel functionality as a hosted
service. See [commercial plans and partner scenarios](https://github.com/Citadel-P/Citadel/blob/main/COMMERCIAL-LICENSING.md)
for examples and separately negotiated permissions. A Team key alone is not
a hosting or reseller agreement.

The official project welcomes Community fixes and reserves paid-feature and
licensing changes for maintainer review. Review approval alone does not waive
ELv2. See [CONTRIBUTING.md](https://github.com/Citadel-P/Citadel/blob/main/CONTRIBUTING.md).

Third-party material retains its own licenses. Known bundled-data notices
include [Unicode-3.0](https://github.com/Citadel-P/Citadel/blob/main/src/features/alerts/LICENSE.unicode)
and [MIT](https://github.com/Citadel-P/Citadel/blob/main/src/features/identity/src/authentication/common-passwords.LICENSE).
Official Core and Agent images carry the root license and these known notices;
they are not a complete inventory of third-party obligations.

Source and license links remain available on the sign-in/setup screens and
in the account menu. Commit-specific source links are retained for transparency,
not as a network-source-disclosure requirement imposed by ELv2. Third-party
license obligations still apply. This update does not revoke rights already
granted for earlier copies; see the [legal distribution checklist](https://github.com/Citadel-P/Citadel/blob/main/docs/LEGAL-RELEASE-CHECKLIST.md)
for ownership, earlier public snapshots and release review.

## Edition Comparison

| Area | Community | Team | Enterprise |
| --- | --- | --- | --- |
| Availability | Included without a paid product key | Paid license | Not currently generally available |
| Intended for | Individuals, evaluation, labs, and manually operated environments | Teams operating shared or production environments | Organizations requiring centralized governance, resilience, and formal assurance |
| Resource counts | No license-enforced limits | No license-enforced limits | No license-enforced limits |
| Users and teams | Included | Included | Included |
| Custom access control | Existing custom access and retained Service Accounts remain visible and removable after downgrade, but expansion, machine authentication, and run-as use are unavailable | Custom roles, scoped access, non-human identities, API tokens, and run-as execution | Team capability plus future organizational identity and access governance |
| Authentication | Local authentication, MFA, and existing OIDC | Same as Community | Future enterprise identity lifecycle, such as SCIM and enforced organizational SSO |
| Automation actions | Define, test, and run manually | Manual, scheduled, and webhook-triggered execution | Team capabilities plus future approval and change-control policies |
| Backups | Define policies, run on demand, and restore | On-demand, scheduled, and webhook-triggered backups | Team capabilities plus future governed and validated recovery workflows |
| Webhooks and GitOps | Receive and authenticate repository webhooks, synchronize source, and show pending updates | Automatically build, deploy, apply, run an action, or start a backup from a webhook | Future approval and organization-wide deployment policy |
| Build execution | Manual builds on existing Local, Agent, and Edge Agent platforms; build-pool configuration remains manageable | Execute builds through external build pools | Future governed elastic capacity and organizational build policy |
| Alerts | Seeded system rules, in-app notifications, external channels, and external delivery | Custom rules, quiet hours, cooldowns, thresholds, required matches, severity, and resource scoping | Future escalation, immutable delivery, and compliance integration |
| Drift and updates | Manual drift checks, reconciliation, updates, and rollback | Continuous drift monitoring, opt-in reconciliation, and auto-update safeguards | Future fleet-wide policy and remediation controls |
| Activity and audit | Local activity history | Local activity history | Future immutable audit delivery and compliance reporting |
| Citadel control plane | Single-instance operation | Single-instance operation | Future highly available control plane and validated recovery |
| Support | Documentation and community support | Commercial support according to the subscription | Future enterprise SLA, upgrade assistance, and incident response |

Enterprise entries describe the intended edition boundary, not currently
released features. The License page shows only capabilities implemented by the
running Citadel version and included in the installed license.

## How Citadel Decides Access

Citadel enforces signed capabilities, not the edition name by itself. The
edition identifies the commercial package, while the capability list in the
installed license determines which paid workflows are enabled.

| Installed license | Required capability included | Result |
| --- | --- | --- |
| Team | Yes | The workflow is enabled |
| Team | No | The workflow remains unavailable |
| Enterprise | Yes | The workflow is enabled |
| Enterprise | No | The workflow remains unavailable |

Enterprise is recognized by the license format and can carry the currently
shipped Team capabilities. It does not automatically inherit capabilities that
are absent from its signed capability list.

All paid capabilities implemented in the current release are Team
capabilities. Citadel therefore shows `Team` on current locked-feature
indicators. The lock and edition label explain the normal minimum subscription;
they are not the authorization check. The License page remains the authority
for the capabilities included in a particular installed license.

Citadel will show an `Enterprise` indicator only after a concrete
Enterprise-only feature, capability, backend enforcement boundary, downgrade
behavior, and tests have been implemented. There are no Enterprise-only runtime
checks in the current release.

## Choosing Or Changing An Edition

### Move From Community To Team

Team is appropriate when manual operation is no longer sufficient or several
people need more precise access control.

Consider Team when you need any of the following:

- non-human API identities for CI/CD runners or external integrations
- stable Service Account run-as identities for Automation Actions or Backup
  Policies
- custom roles instead of only Admin, Operator, and Viewer
- different permissions for particular resources or teams
- backups that run without an administrator starting them
- automation triggered by a schedule or webhook
- webhook-triggered builds, deployments, and stack applies
- builds that execute through an external build pool
- custom alert rules beyond Citadel's seeded system rules
- quiet hours, cooldowns, thresholds, required matches, severity changes, or
  resource-specific alert conditions
- continuous drift detection or opt-in automatic updates
- commercial support for a production installation

Resource growth alone is not a reason to upgrade. Community and Team do not
limit the number of platforms or managed resource records.

### Move From Team To Enterprise

Enterprise is not currently generally available. Once released, it will be
appropriate for organizations that need capabilities such as:

- automated identity provisioning and deprovisioning
- organization-wide access and deployment policy
- approval workflows, change windows, and separation of duties
- immutable audit delivery or formal compliance reporting
- a highly available Citadel control plane
- validated upgrades, production SLAs, and incident response

Do not select Enterprise today based on these requirements; they are product
direction until the corresponding Citadel features are released.

### Move From Team To Community

Returning to Community can make sense when an installation no longer needs
unattended paid workflows, custom access configuration, or commercial support.

Before removing the Team license, review:

- custom roles and resource overrides that remain assigned
- automation schedules and webhook triggers that will pause
- backup schedules and webhook triggers that will pause
- webhook-triggered builds, deployments, and stack applies that will pause
- queued external build-pool runs that will not start
- custom alert rules that will pause; notification channels and seeded-rule
  delivery remain active
- continuous guardrails and auto-update settings that will pause
- Service Account API tokens and run-as bindings that will be suspended

Citadel retains this configuration and continues enforcing existing custom-role
assignments. Manual operation, recovery, restore, history, and data access
remain available. Reinstalling an eligible Team license reactivates persisted
paid configuration.

### Move From Enterprise To Team

This transition applies only after Enterprise is released. It will be
appropriate when an organization no longer needs Enterprise identity,
governance, high-availability, compliance, or support assurances.

Team capabilities included in the replacement license remain active.
Enterprise-only workflows will follow the same non-destructive pause and
recovery rules documented for other license transitions.

## Editions

### Community

Community includes the core Citadel product:

- all supported platform and connector types
- platforms, stacks, deployments, and builds on existing Local, Agent, and Edge
  Agent platforms
- build-pool definitions and configuration, without build-pool execution
- registries, Git repositories, webhook reception, source synchronization, and
  pending-update detection
- local authentication, MFA, and OIDC login
- users, teams, and the built-in Admin, Operator, and Viewer roles
- backup repositories and backup policy definitions
- on-demand backup and restore
- automation action definitions, testing, and manual execution
- built-in alerts, notification channels, and external delivery from seeded
  system rules
- manual drift checks, updates, reconciliation, and rollback
- access to resource history, logs, alerts, and activities

Community has no license-enforced platform, user, backup-policy, or automation
count limits.

Technical safeguards such as request-size limits, operation concurrency,
timeouts, and available storage still apply. These protect the installation and
are not edition quotas.

### Team

Team adds capabilities for unattended production operation and advanced
collaboration.

| Capability | What Team Adds |
| --- | --- |
| Custom access control | Custom roles, custom permissions, resource overrides, scoped access grants, Service Accounts, API credentials, and Service Account run-as execution |
| Automated operations | Schedules and webhooks that execute automation actions, backups, builds, deployments, or stack applies |
| Advanced alerting | Custom alert rules and advanced conditions including quiet hours, cooldowns, thresholds, required matches, severity, and resource scoping |
| Operational guardrails | Continuous drift monitoring, opt-in automatic reconciliation, and resource auto-update safeguards |
| Elastic build execution | Builds dispatched through external build pools that provide dedicated, remote, or ephemeral builder compute |

A Team license can contain a subset of these capabilities. The License page is
the authority for what is included in the installed license.

A standard Team subscription may include one production installation and one
non-production installation for upgrade and recovery testing. Each installation
has a different instance-bound license; one license value cannot be reused
across both installations.

### Enterprise

Enterprise is reserved for future organizational governance, identity
lifecycle, high availability, and production-assurance capabilities.

Enterprise is not currently generally available. Citadel does not currently
provide SCIM provisioning, a highly available Citadel control plane, immutable
audit delivery, or compliance reporting. These features must not be assumed
from an Enterprise edition label.

An Enterprise license can enable currently shipped Team workflows when their
capabilities are explicitly included. The Enterprise edition name alone does
not enable those workflows or any future Enterprise functionality.

## Features That Are Never Locked

The following operations remain available regardless of license status:

- User sign-in and enforcement of existing permissions
- MFA and existing OIDC login
- reading customer data and configuration
- viewing logs, history, activities, and existing alerts
- creating, testing, and managing notification channels
- routing seeded system alert rules to notification channels
- disabling or deleting resources
- removing custom access assignments
- manually starting a backup
- restoring a backup
- manually rolling back or reconciling a resource
- completing an operation that was already running when the license changed

Citadel does not delete or rewrite configuration when a paid capability becomes
unavailable.

## Open The License Page

An administrator can open:

```text
Settings > License
```

The page shows:

- effective edition
- license status
- licensed edition, when different from the effective edition
- Citadel instance ID
- customer and license identifiers
- activation, expiration, and grace-period dates
- included capabilities
- validation warnings

The raw signed license is never displayed after installation.

Only administrators with License permissions can view the license request,
install a license, replace it, or remove it.

## Request A License

An offline license is bound to one Citadel instance.

To request one:

1. Open `Settings > License`.
2. Find the `License Request` section.
3. Copy the request data.
4. Send it to the Citadel license issuer through the agreed support or sales
   channel.
5. Receive a signed `.citadel-license` value for that instance.

The request contains:

- product name
- Citadel instance ID
- Citadel Core release version in `MAJOR.MINOR.PATCH` format, such as `1.0.0`
- request generation time

It does not contain credentials, user information, platform addresses, host
identifiers, resource inventory, or resource counts.

## Install Or Replace A License

To install a license:

1. Open `Settings > License`.
2. Paste the complete signed license value into `Install License`.
3. Select `Install`.
4. Confirm that the expected edition, customer, dates, and capabilities appear.

Installing a new license replaces the previously installed license atomically.
Citadel verifies the signature, instance binding, dates, and payload before
storing it.

A license for another Citadel instance is rejected. Do not edit the license
text: changing any signed content invalidates its signature.

## License Statuses

| Status | Meaning |
| --- | --- |
| Community | No paid license is installed |
| Valid | The paid license is active |
| Grace Period | The license has expired, but paid capabilities remain active until the grace end |
| Not Yet Valid | The license is authentic but its activation time has not arrived |
| Expired | The license and grace period have ended |
| Invalid | The license is malformed, has an invalid signature, or contains invalid data |
| Instance Mismatch | The license belongs to a different Citadel instance |
| Unsupported Schema | This Citadel version cannot read the license format |
| Unknown Signing Key | This Citadel version does not recognize the key used to sign the license |

For `Not Yet Valid`, `Expired`, `Invalid`, `Instance Mismatch`,
`Unsupported Schema`, and `Unknown Signing Key`, Citadel uses Community
capabilities.

## Expiration And Grace Period

Paid capabilities continue normally during the grace period. The License page
shows when the grace period ends.

After the grace period:

- paid schedules stop creating new runs
- paid webhooks may still be received and authenticated, but they cannot start a
  mutating operation
- queued paid-trigger runs that have not started do not begin
- queued external build-pool runs do not lease or provision builder capacity
- external builds already running may finish or be cancelled
- custom alert rules are paused
- notification channels and external delivery from seeded system rules continue
- continuous guardrails and auto-update are paused
- existing custom-role authorization continues to be evaluated
- Service Account credentials and new Service Account run-as work are suspended
  without revoking tokens or changing saved bindings
- administrators can inspect, disable, revoke, and archive retained Service
  Accounts
- paid configuration remains stored and visible
- operations already running are allowed to finish

Affected configuration is shown as `Paused by license`. Citadel does not change
the resource's configured enabled state.

Installing a replacement license reactivates eligible configuration without
requiring every schedule, rule, or resource to be edited.

For Service Accounts, eligible means the account remains enabled and
non-archived and its token is unrevoked and either unexpired or non-expiring.
Citadel does not change token state merely because the capability became
unavailable.

## Removing A License

Removing the installed license returns Citadel to Community immediately.

The same non-destructive rules used for expiration apply:

- configuration is retained
- paid triggers are paused
- recovery and manual operations remain available
- existing authorization assignments continue to be enforced

Removing a license cannot be used to delete the license file from another
installation or revoke a license remotely. Citadel licensing is offline.

## Legacy Business Licenses

Older Citadel licenses use the Business edition and numeric resource limits.

During the licensing migration:

- a valid or grace-period Business license is treated as Team
- all shipped Team capabilities are enabled
- the old numeric limits no longer restrict resource counts
- the License page shows a legacy-license warning
- the license should be replaced with a Team license at renewal

An expired or invalid legacy license provides only Community capabilities.

## Moving Or Restoring Citadel

The Citadel instance ID is stored in the Citadel database.

Restoring the complete Citadel database preserves the instance ID, so the
installed license remains bound to the restored instance. Creating a new
database creates a new instance ID and requires a rehosted license.

Do not manually copy an installed license to an unrelated Citadel database. It
will fail instance validation.

## Offline Operation And Privacy

Citadel does not call a licensing server to validate an installed license.

The license is verified locally using a public signing key embedded in Citadel
Core. The signed license contains safe customer and entitlement metadata, but it
is not encrypted. Protect the license file as administrative configuration and
do not publish it.

Citadel never returns the raw installed license through the API, activity
history, logs, or frontend state.

## Troubleshooting

### The license reports Instance Mismatch

The license was issued for a different Citadel database instance. Copy a new
license request from the current installation and request a rehosted license.

### The license reports Unknown Signing Key

The Citadel version is older than the key used by the issuer. Upgrade Citadel to
a version that contains the matching public verification key.

### The license reports Unsupported Schema

Upgrade Citadel Core to a version that supports the license format.

### Automated Operations Stopped

Check:

- license status
- grace-period end
- whether `Automated Operations` is included
- whether the individual schedule or webhook is configured and enabled

Manual automation, backup, and restore operations remain available.

### A Webhook Is Accepted But Does Not Execute

Community can receive and authenticate repository webhooks, synchronize source,
and report an available update. Starting a build, deployment, stack apply,
automation action, or backup from a webhook requires `Automated Operations`.

### A Build Pool Is Ready But Cannot Run A Build

Community can create, inspect, test, update, disable, and delete build-pool
configuration. Executing a build through an external build pool requires
`Elastic Build Execution`. Manual builds on existing Local, Agent, and Edge
Agent platforms remain available.

### A Custom Role Still Applies After Expiration

This is intentional. Citadel continues evaluating existing custom roles so a
license transition cannot unexpectedly change authorization or lock out
administrators.

Administrators can remove assignments, reduce permissions, or delete custom
roles. Creating new custom access configuration requires the corresponding
Team capability.

### A Service Account Token Returns Unauthorized

Check whether the installed license includes **Custom access control** and remains
active or in its grace period. Also verify that the account is enabled and the
token is unrevoked and either unexpired or non-expiring.

Citadel intentionally returns the same Unauthorized response for invalid and
license-suspended machine credentials. An administrator can see the exact
license state from **Settings > License** and inspect retained accounts under
**Settings > Access > Service Accounts**.


