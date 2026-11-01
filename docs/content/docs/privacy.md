---
title: Documentation privacy
description: Optional analytics on the Citadel documentation website.
---

## Optional documentation analytics

The published documentation at `docs.citadelplane.com` offers optional,
self-hosted Umami analytics to help us understand which guides people read,
whether search helps them find a page, and which pages need improvement.
Collection starts only after you select **Accept**. Declining does not change
access to the documentation. **Analytics preferences** in the documentation
sidebar (or the API reference header) lets you change or withdraw your choice.

We send only the public page path, its fixed documentation title, the docs
hostname and a public website identifier to `analytics.citadelplane.com`.
After consent, we also measure these limited interactions:

- A completed search: the final result count when you close the search dialog or
  select a result, and whether it had no results. Empty, unfinished and failed
  searches are excluded.
- A search result opened: only the selected public page path, without its query
  string or heading fragment.
- Page helpfulness: your Yes/No response to **Was this page helpful?** on that page.
  This optional prompt appears only while analytics consent is active. It accepts
  one response per page until you reload; responses are sent on a best-effort
  basis and are not retried if collection fails.

We do not send search text, query strings, URL fragments, referring URLs,
form values, account identifiers or the contents of your Citadel installation.
There is no session recording. These statistics are separate from customer
accounts and from analytics on the main Citadel website.

Umami processes connection information, including IP address and browser user
agent, to derive visit and device/geographic statistics. Raw IP addresses are not
stored in its analytics tables. Hosting and network providers still process
network traffic; this is not a claim that the connection itself is anonymous.

## Your choice

Your browser stores only your consent choice and its expiry for up to 180 days.
The choice applies to this docs origin and synchronizes between its open tabs.
When browser storage is unavailable, the choice lasts only in the current page
session. We respect enabled Do Not Track and Global Privacy Control signals even
if you previously accepted. Blocking analytics does not prevent reading the docs.

## Retention and contact

Analytics events are removed by a daily job after 90 days; collection can exceed
that cutoff by one scheduling interval. Private local database backups are kept
for seven days. Withdrawing consent stops new collection; it does not identify
or erase earlier aggregate visits.

For operator identity, hosting details and privacy enquiries, see the
[Citadel privacy policy](https://citadelplane.com/privacy).
