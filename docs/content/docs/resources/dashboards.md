---
title: Dashboards
description: Save dashboard definitions and choose personal and instance defaults through the API.
---

Signed-in users can save private dashboards. Only the owner can open or edit a private dashboard. Administrators can also create instance-shared dashboards that all signed-in users can read. Team adds restricted dashboards and delegated editing through Dashboard permissions. Access to a dashboard does not grant access to the resources it references.

To share a restricted board on Team, open the user or team in **Access**, choose **Grant Access**, and select **Dashboard**. Choose **Read** to let them open the board or **Write** to let them edit it. Save the override and the user or team form.

Use `GET /api/v1/dashboards` to list boards you can read and `GET /api/v1/dashboards/{id}` to load a definition. When saving an edit or deleting a dashboard, send the revision you last read. If another editor saved first, the API returns a conflict. Read the latest definition before reapplying your change.

A personal default takes precedence over the instance default. If the personal default is deleted or you lose access, Citadel resolves the next default you can read. The API returns a null dashboard ID if neither default is available.
