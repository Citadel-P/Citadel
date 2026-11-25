---
title: Dashboards
description: Open saved dashboards, manage their audience, and choose defaults.
---

Signed-in users can save private dashboards. Only the owner can open or edit a private dashboard. Administrators can also create instance-shared dashboards that all signed-in users can read. Team adds restricted dashboards and delegated editing through Dashboard permissions. Access to a dashboard does not grant access to the resources it references.

To create a board, open **Dashboards → Create dashboard**, enter a name, and choose a template. **Blank** starts empty. **Overview** covers resources, alerts, backups and activity. **Maintenance** adds upcoming schedules and known release information. **Infrastructure and capacity** and **Application** ask you to choose a resource for their charts. The new board is private unless an administrator chooses a shared audience during creation.

Overview shows the latest CPU and memory observations for your visible platforms and a count of the containers they report. Select a platform or alert to open its details. Workload rows show runtime status, backup coverage, and results from up to 16 observed container probes. Hover over the probe indicator to see the breakdown. **Backup activity** lists completed backup events. Select one to see its outcome. Backup and recent activity refresh about once a minute while the board is visible.

Saved tag filters match resources that carry all the selected tags. Backup activity follows the backup policy’s tags. Alerts and recent activity follow the tags of their owning resource and appear only when you can read both the event and the owner. Instance-wide events have no tagged owner. Changing a resource’s tags updates which past events appear on the dashboard.

To share a restricted board on Team, open the user or team in **Access**, choose **Grant Access**, and select **Dashboard**. Choose **Read** to let them open the board or **Write** to let them edit it. Save the override and the user or team form.

Open **Dashboards** and select a board by name. On a shared or restricted board, administrators can choose **Dashboard options → Sharing** and confirm the new audience. Restricting a board requires Team. Sharing a restricted board with everyone also makes its saved notes and links visible to every signed-in user. Existing user, team and role grants keep their assigned levels. Personal boards always stay private, including boards owned by administrators.

Select the dashboard name to switch boards. Choose **Edit dashboard**, then **Add widget** to open the library. Select a widget title to open its settings. Close the panel for a full-width canvas. Drag the grip in the header to move a tile, drag its corner to resize it, or set its column, row, width and height in **Widget settings**. The canvas previews edits as you make them. **Apply** closes the widget settings, and **Cancel settings** restores the previous widget. **Save changes** saves the whole board. If someone else saves first, your draft stays open. A board supports 24 widgets, including up to eight metric charts.

Use the tag filter and history range to change your current view. **Pause** freezes it. **Refresh** updates a paused board once. In the editor, the same controls set defaults for the saved board. A pinned chart keeps its selected resource when you change filters. If the resource falls outside the filter or you lose access, the chart shows that state.

Metric charts use the selected resource’s statistics and support the same history windows, from 15 minutes to seven days. New charts use a gradient. Choose **Line**, **Area**, or **Gradient** in widget settings. Container and workload memory charts offer **Breakdown** and **Working set** when measurements are available. Change **History range** in any active chart to update the dashboard’s charts together. When a stack has several services, choose one in the chart. For stacks with more than eight current or recorded service names, use **Full statistics**. Upcoming schedules and instance updates require administrator access.

Home opens your personal dashboard, then the instance default if you can read it. If neither is available, Home shows the built-in Overview. **Dashboard options → Save a copy** lets you name and save your own Overview. Opening a dashboard or saving a copy leaves your home preference unchanged.

On a saved dashboard, choose **Dashboard options → Set as home** to open it whenever you visit Home. Administrators can also set a shared or restricted dashboard as the **Instance default**. To clear a choice, open **Dashboards → Reset home**. Resetting your own home restores the instance default. Resetting the instance default restores the built-in Overview for people without a personal default.
