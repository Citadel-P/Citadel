---
title: "Access control"
description: "Manage Citadel users, Teams, Roles, Service Accounts, permissions, and resource overrides."
---

Citadel access control is built from Users, Teams, and Roles.

- **Users** are people who sign in to Citadel.
- **Teams** group Users and Service Accounts so they can inherit the same access.
- **Roles** define reusable permission sets.
- **Resource access** grants additional access to a specific resource.

Permissions are additive. Prefer a purpose-specific Role and narrowly scoped
resource access instead of assigning the built-in Admin Role.

## Review Access Changes

The **Activities** tab on a User, Team, or Role records the administrative
changes that affect that resource:

- creation;
- configuration or access-assignment changes;
- rename;
- deletion.

The main **Activities** page can be filtered by User, Team, or Role. These
identity activity records are visible only to administrators.

Activity records contain the affected enabled state, assignment identifiers,
and permission levels needed for an audit. Citadel does not record passwords,
password hashes, MFA secrets, API tokens, or other credential values in these
events.


