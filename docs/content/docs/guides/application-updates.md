---
title: "Update applications"
description: "Review image and Git changes, deploy deliberately, and handle drift or rollback."
---

Use this workflow for applications managed by Citadel. To update Citadel itself,
follow [Upgrade and rollback](/docs/operations/upgrade-and-rollback).

## Choose what to check

| Source | Check and apply path |
| --- | --- |
| Deployment with an external image tag | **Check for updates**, review the result, then **Redeploy**. |
| Web Editor Stack | **Check for updates** compares supported service image tags; deploy the Stack to apply the intended change. |
| Git Stack | **Check for updates** looks for a relevant newer commit; review source and deploy. |
| Managed Swarm Service with an external tag | **Check for updates**, then **Apply**; Scale and Restart Tasks retain the applied image. |
| Citadel Build output | Run the Build, review its artifact, then deploy the consuming resource. Registry tag checks do not replace this workflow. |

Manual checks remain available with periodic updates disabled. They record update
state without deploying. **Updates available** filters on the Deployment and
Stack lists show detected changes, excluding unknown and failed checks.

## Review and apply

1. Check the current application's health and record the applied image digest,
   Git commit, or Stack release. Keep a recoverable backup of persistent data
   before changes that can migrate it.
2. Run **Check for updates** on the saved resource. Review the result and its
   timestamp. A tag check detects a changed digest under the same tag; it does
   not select a newer version tag for you.
3. For a Git Stack, inspect changed source and watched paths. A pinned commit
   stays pinned. A manual Stack check does not run repository hooks or update
   sibling Stacks; repository **Sync** can do both through their configured policies.
4. Save any intended configuration change, then deploy or apply. Keep the
   progress view connected and inspect the final operation result.
5. Verify current containers or Tasks, logs, and the application endpoint. Keep
   the previous artifact and data backup until the new version is verified.

Use [Git update and recovery instructions](/docs/resources/stacks/git#review-and-deploy-an-update)
or the [Build-to-deployment workflow](/docs/resources/builds#deploy-the-image)
for those sources. To change a fixed image version, edit its tag explicitly;
digest-pinned images do not follow a moving tag.

## Enable automation after the manual path works

Start with **Notify Only** and attach an [alert channel](/docs/resources/alert-rules).
Enable automatic deployment only for the resources and tags you intend to follow.
Automatic operations require the capabilities described in each resource guide.
An accepted webhook or successful Build is not proof that the application is ready.

Choose the scope deliberately: Docker Standalone Stacks support service-scoped
updates; Swarm Stacks apply the complete Stack and use Swarm's rollout settings.
See [Stack update behavior](/docs/resources/stacks/web-editor#update-behavior).

## Handle drift separately

Drift means runtime state differs from the Stack's desired state. A newer image
or Git commit is an update, even when the existing runtime has no drift.

For a Docker Standalone Stack, inspect its drift report before selecting
**Reconcile drift**. Reconciliation can start stopped or resume paused containers;
extra-container removal follows the configured policy. Structural differences
require a reapply. Reconciliation requires **Operational Guardrails** and an
enabled drift policy. Consider whether a stop or pause was intentional before
repairing it. Swarm handles Task convergence separately.

## Recover an unsuccessful update

Read the failed operation and current runtime state before retrying. A Stack can
use a previously healthy release, but rollback does not restore volumes or old
secret values. Mutable image tags may also resolve differently. Git Stacks need
an explicit commit selection for exact source recovery. See
[Stack rollback](/docs/resources/stacks/web-editor#rollback) and
[Git rollback](/docs/resources/stacks/git#rollback).

For a Deployment, restore the intended saved configuration and previous image
reference, then redeploy. Verify application data compatibility separately;
replacing a container does not undo a database migration.

If checking fails, verify source credentials, Platform connectivity, and that a
successful deployment established the digest or commit to compare. A Stack may
retain its earlier check result after a failed check; read the error and timestamp
before treating an older result as current.
