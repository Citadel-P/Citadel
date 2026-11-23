---
title: "Resource tags"
description: "Organize and locate Citadel resources with tags."
---

Tags group related resources across Citadel. Use names such as `Prod`,
`Staging`, `Database`, or `Customer-A` to describe environment, purpose, or owner.
Do not put secrets or credentials in tags.

## Who Can Manage Tags

Administrators can manage all tags. Other users need global Tag Write
permission to create tags, or Write permission on an existing tag to rename,
recolor, or delete it. The tag catalog shows the tags you can read. **Resources you can access** counts the resources you can open that carry each tag.

Assigning tags requires Write permission on the resource being tagged.

## Create A Tag

Open **Configuration → Tags**.

Create a tag with:

- `Name`: display name, such as `Prod`
- `Color`: hex color, such as `#EF4444`

Tag names are unique without case sensitivity. `Prod`, `prod`, and `PROD` are treated as the same tag name.

Colors use six-digit hex format, such as `#22C55E`.

## Assign Tags To A Resource

You can assign tags while creating a supported resource or from the resource edit flow.

Supported resources:

- deployments
- stacks
- platforms
- git repositories
- registries
- scripts
- backup policies
- build projects
- build pools
- managed Swarm Services

Select one or more tags from the tag selector and save the resource.

If you do not have write permission for a resource, you can see its tags but cannot change them.

## Replace Tags

Editing tags replaces the full assigned tag set for that resource.

Example:

```text
Current tags: Prod, Backend
New selection: Prod, Customer-A
Saved tags: Prod, Customer-A
```

The removed tag is no longer assigned to that resource, but it still exists in the global tag catalog.

## Filter By Tags

Use [global search](/guides/finding-resources) to find a resource by name
across Citadel, then use Tag filters on its list to narrow related resources.

Selecting `Prod` and `Backend` shows resources carrying both tag names. Changes to capitalization keep matching. If a name is deleted or renamed, its filter stays selected. You can remove it, replace it, or keep it to match resources tagged with that name again.

The filter is reflected in the page URL, so filtered views can be bookmarked or shared with another user who has access to the same resources.

## Display Rules

Tags appear in resource lists and details. A `+N` indicator reveals additional
tags when the list is too long to show inline.

## Delete A Tag

Users with Write permission on a tag can delete it from **Configuration → Tags**.

Deleting a tag removes it from every resource that uses it. The resources themselves are not deleted or changed otherwise.

Use delete when the tag should no longer be available anywhere. If you only want to remove a tag from one resource, edit that resource's assigned tags instead.
