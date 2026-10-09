---
title: "Find resources and activity"
description: "Use global search, list filters, and activity details to find the right resource or operation."
---

Use **Search resources** in Citadel's header, or press **Ctrl+K** on Windows/Linux
or **⌘K** on macOS. The shortcut is inactive while typing in an editor, input, or
another dialog; use the header button in that case.

## Search across Citadel

1. Enter at least two characters from the resource name. Search is case-insensitive.
2. Check the result's type, status, and parent context before opening it.
3. Refine the name if there are many matches; the popup shows a limited selection
   per resource type. With an empty query, select a category to open its list.

Search includes Platforms, Stacks, Deployments, managed Swarm Services, Git
Repositories, Registries, Automation Actions, Backup Policies and Repositories,
Build Projects, and Build Pools. Some types also match a useful field such as a
Platform address, Git URL, Registry host, or Build branch.

Results require Read access. A linked parent is shown only when you can also
read it. If a resource is missing, try its list page and review
[access assignments](/docs/guides/access-control#verify-the-teammates-access)
before assuming it was deleted.

## Narrow a resource list

Use a list's search and available Platform or [Tag filters](/docs/guides/resource-tags)
to narrow the results. Clear filters when a known resource appears to be missing.
On Deployments and Stacks, **Updates available** shows detected updates; an empty
result does not prove that every resource has been checked successfully.

Global search does not search container logs, Compose contents, secret values,
or run history. Open the owning Platform for Docker containers, images,
networks, and volumes. Use the resource's **Runs**, **Releases**, or **Activities**
view for its history.

## Investigate an operation

Find the resource and time in **Activities**, then open the event to inspect its
details. Use the event's copy-link control when sharing it with a teammate who
has access. For a failed job, also inspect its run logs; for an alert, follow
[the investigation workflow](/docs/resources/alert-rules#respond-to-an-alert).

Saved configuration, a recorded operation, and current runtime state answer
different questions. Check the running workload and application endpoint before
retrying a deployment whose progress connection was interrupted.
