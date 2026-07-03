# Resource Tags

Tags help organize deployments, stacks, platforms, and git repositories.

Use tags for labels such as:

- `Prod`
- `Dev`
- `Staging`
- `Customer-A`
- `Experimental`

Tags are global. An admin creates the available tags, and users assign those existing tags to resources they can edit.

## What Tags Are For

Use tags when you want to group and find resources across Citadel.

Good examples:

- environment: `Prod`, `Staging`, `Dev`
- workload type: `Backend`, `Database`, `Monitoring`
- customer or project: `Customer-A`, `Internal`
- lifecycle: `Experimental`, `Legacy`

Avoid using tags for secrets, credentials, or values that should not be visible to other users.

## Who Can Manage Tags

Admins can:

- create tags
- rename tags
- change tag colors
- delete tags

Non-admin users can view the tag catalog and assign tags to resources when they have permission to edit those resources.

## Create A Tag

Open the global `Tags` page.

Create a tag with:

- `Name`: display name, such as `Prod`
- `Color`: hex color, such as `#EF4444`

Tag names are unique without case sensitivity. `Prod`, `prod`, and `PROD` are treated as the same tag name.

Colors must use `#RRGGBB` format.

Valid:

```text
#22C55E
#EF4444
#3B82F6
```

Invalid:

```text
red
#FFF
22C55E
```

## Assign Tags To A Resource

You can assign tags while creating a supported resource or from the resource edit flow.

Supported resources:

- deployments
- stacks
- platforms
- git repositories

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

Supported list pages include a tag filter:

- Deployments
- Stacks
- Platforms
- Git repositories

Select one or more tags to filter the list.

Filtering uses OR behavior. If you select `Prod` and `Backend`, Citadel shows resources that have either `Prod` or `Backend`.

The filter is reflected in the page URL, so filtered views can be bookmarked or shared with another user who has access to the same resources.

## Display Rules

Resource lists show tag chips in a Tags column.

If a resource has many tags, Citadel shows the first few tags and a `+N` indicator for the rest.

Resource detail pages show tags near the resource title or metadata area.

## Delete A Tag

Admins can delete a tag from the global Tags page.

Deleting a tag removes it from every resource that uses it. The resources themselves are not deleted or changed otherwise.

Use delete when the tag should no longer be available anywhere. If you only want to remove a tag from one resource, edit that resource's assigned tags instead.

## Practical Examples

### Environment Tags

Create:

```text
Prod
Staging
Dev
```

Assign `Prod` to production deployments and stacks. Use the `Prod` filter when checking production resources.

### Customer Tags

Create:

```text
Customer-A
Customer-B
Internal
```

Assign customer tags to resources that belong to a customer or internal project. Use the tag filter to review one customer's resources across deployments, stacks, platforms, and repositories.

### Temporary Work

Create:

```text
Experimental
```

Assign it to trial resources. Remove the tag when the resource becomes permanent, or filter by the tag to clean up old experiments.
