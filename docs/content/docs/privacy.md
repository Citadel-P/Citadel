---
title: Documentation privacy
description: Optional analytics on the Citadel documentation website.
---

## Optional documentation analytics

The published documentation at `docs.citadelplane.com` offers optional,
self-hosted analytics to help us understand which guides people read,
whether search helps them find a page, and which pages need improvement.
Nothing is collected before a choice. **Reject non-essentials enables basic
measurement**: public article views, installation copies, API download requests
and helpfulness votes, without custom event properties. It excludes the privacy
page and all search events. **Accept all** enables the existing wider measurement
described below, including search and fixed event details.

**Preferences** in the notice, or **Analytics preferences** in the
documentation sidebar (or API reference header), opens a dialog with Basic, Full
and **Off** choices. Basic is preselected if you have no saved choice, but nothing
is collected until you confirm. Select a level and click **Save preferences**
to apply it. Closing the dialog leaves your current choice unchanged.
None of these choices changes access to the documentation.

We send only the public page path, its fixed documentation title, the docs
hostname and a public website identifier to `analytics.citadelplane.com`.
Both basic and full analytics can measure these interactions:

- Page helpfulness: your Yes/No response to **Was this page helpful?** on that page.
  This optional prompt appears only when the chosen level permits measurement on that page. It accepts
  one response per page until you reload; responses are sent on a best-effort
  basis and are not retried if collection fails.
- Installation command copies: a successful copy from a marked installation or
  quick-start example. Full mode includes a fixed step label (download, start, or start with TLS);
  basic mode omits it.
  We do not send the copied commands. A copy does not prove a successful installation.
- API schema download requests: selecting **Download OpenAPI schema** in the API
  reference. This records the request, not a completed download.

Full analytics additionally measures the privacy page and these search interactions:

- A completed search: the final result count when you close the search dialog or
  select a result, and whether it had no results. Empty, unfinished and failed
  searches are excluded.
- A search result opened: only the selected public page path, without its query
  string or heading fragment.

We do not send search text, query strings, URL fragments, referring URLs,
form values, account identifiers or the contents of your Citadel installation.
There is no session recording. These statistics are separate from customer
accounts and from analytics on the main Citadel website.

Our analytics system processes connection information, including IP address and browser user
agent, to derive visit and device/geographic statistics. Raw IP addresses are not
stored in its analytics tables. Hosting and network providers still process
network traffic; this is not a claim that the connection itself is anonymous.

In both modes, our analytics system associates events using a pseudonymous visitor/session ID
derived from the website ID, IP address, browser user agent and a monthly salt.
This lets us estimate returns within the same UTC calendar month. Recognition
resets at the next month. Separate visit IDs use hourly grouping in our integration,
so they are not exact browser sessions. Changes to your network or browser
can make you appear as a new visitor; people sharing a network and browser type can
be grouped together. These estimates cover only traffic permitted by your choice
and browser signals; blocked collection and disabled analytics are not counted.
We do not store a visitor identifier in your browser or link documentation visits
to visits on the main Citadel website.

## Your choice

Your browser stores only your consent choice and its expiry for up to 180 days.
The choice applies to this docs origin and synchronizes between its open tabs.
An expired choice disables collection and prompts again.
When browser storage is unavailable, the choice lasts only in the current page
session. Global Privacy Control prevents Basic analytics; choosing **Accept all**
or saving **Full** in preferences overrides that signal for this docs site while
your choice remains valid, including after reload. Switching to Basic or Off, or
letting Full expire, removes the override. Do Not Track prevents collection in
both modes. Browser or extension request blocking can still prevent delivery.
Blocking analytics does not prevent reading the docs.

## Retention and contact

Analytics events are removed by a daily job after 90 days; collection can exceed
that cutoff by one scheduling interval. Private local database backups are kept
for seven days. Withdrawing consent stops new collection; it does not identify
or erase earlier aggregate visits.

For operator identity, hosting details and privacy enquiries, see the
[Citadel privacy policy](https://citadelplane.com/privacy).
