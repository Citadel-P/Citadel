---
title: "Community commitment"
description: "What stays free, how Citadel communicates changes, and how you keep control of your installation."
---

Citadel is source-available under Elastic License 2.0 (`Elastic-2.0`),
not OSI-approved open source. Community is free for personal self-hosting and
internal business production use under those terms. A company does not need
Team merely because it is a company.

This is Citadel's product policy for official supported releases that publish
it. It does not amend or replace the source license, waive its restrictions,
create an automatic hosting permission, or promise future open-source
conversion. See [source licensing and editions](/docs/overview/licensing).

## What stays free

**We will not move established Community capabilities behind a paid product
key in later supported Citadel releases.** Paid plans add capabilities rather
than reclaiming workflows already included in Community.

The baseline below records the Community functionality described in the
[edition guide at the reviewed revision](https://github.com/Citadel-P/Citadel/blob/0e65fbe5d49d1f66cb12920a56e8b36375c93167/docs/content/docs/overview/licensing.md#community).
It is the protected minimum when this policy is adopted, not a roadmap of
unimplemented features. Capabilities subsequently shipped and documented as
Community in supported releases extend this minimum. Preview experiments and
unreleased proposals must be identified separately.

| Area | Protected Community baseline |
| --- | --- |
| Hosts and workloads | Supported Docker Standalone and Swarm management; Local, Agent and Edge connections; platforms, deployments, Compose stacks, and manual builds on those platforms |
| Git and registries | Registry and repository configuration, webhook reception/authentication, source synchronization and pending-update detection; manual deployment/apply |
| Identity | Local sign-in, MFA, existing OIDC login, users, teams and the built-in Admin, Operator and Viewer roles |
| Recovery | Backup repository and policy definitions, on-demand backup and restore, manual rollback and reconciliation |
| Automation | Define, test and manually run automation actions |
| Alerts and visibility | Seeded system rules, notification channels and their external delivery, logs, history, activities and existing alerts |
| Build pools | Define, inspect, test and manage pool configuration; execution through external pools is not included |

Community has no product-license-enforced resource-count limits. Growth in
hosts, containers, stacks or users is not itself a reason to buy Team.
Technical capacity, safety limits and authorization checks still apply.
Renaming editions, repackaging a feature or introducing artificial quotas
must not be used to turn the protected baseline into a paid feature.

## What is paid today

The [edition comparison](/docs/overview/licensing#edition-comparison) remains
the reference for implemented behavior in each version.

**Scheduled backups and webhook-triggered deployments still require Team.**
So do other scheduled/webhook-triggered operations, custom access control,
advanced alerting, continuous operational guardrails and external build-pool
execution, as described in that guide. This policy does not unlock them or
change signed entitlements. Considering a more generous free tier is not the
same as shipping one; changes need implementation, tests and release notes
before documentation presents them as available.

## Changes without surprises

Material changes to licensing, paid plans, compatibility or support must have
a dated public notice before the affected release, identifying the old and new
behavior, affected versions, effective date, who is affected and any required
action. Pricing and renewal changes must also follow the customer's applicable
agreement; a changed website is not a substitute for contractual notice.

The README, edition guide, website/pricing copy, release notes and application
must describe the same policy for the same version. Label preview-only behavior
explicitly. Preserve previous terms and release-specific edition information
in version history; do not silently replace history or assert retroactive
revocation of rights already granted for earlier copies.

Security fixes or upstream incompatibility may require a feature to change or
be retired. Explain why and provide a free migration or replacement path where
feasible. Do not sell the former Community workflow back as its paid-only
replacement. Emergency security notices may follow the fix when prior detail
would put users at risk; publish the explanation as soon as it is safe.

This policy does not guarantee that every integration remains available forever,
that unsupported versions receive updates, or that free support has an SLA.
Those limitations are not a way to evade the protected Community baseline.

## Keep control of your installation

Community must not require a vendor-hosted Citadel account, paid activation or
online license validation. Local administrator setup and authentication to your
own installation remain necessary. Pulling images, reaching Git/registries and
other explicitly configured integrations can still require network access.

Paid-license expiry must not delete configuration, weaken existing access
control, block sign-in or remove access to data and history. Preserve the
Community recovery paths: on-demand backup/restore, manual reconciliation and
rollback. Document which unattended paid workflows pause and how to resume
them; do not describe expiry as consequence-free.

Keep documented ways to save Compose files, retain configuration and back up
or restore installation data without a paid entitlement. Documentation must
identify export limitations and migration steps honestly; this is not a claim
that a one-click export for every resource already exists. New portability tools
should be implemented and tested before they are advertised.

## Report a mismatch

Report contradictory terms, an unexpected paywall, or an undocumented migration
requirement through [GitHub Issues](https://github.com/Citadel-P/Citadel/issues).
Include the version and affected workflow, not credentials or private data.
Maintainers should treat a baseline regression as a product defect and track
its resolution publicly, while handling security-sensitive details privately.
