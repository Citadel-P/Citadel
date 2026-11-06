# Citadel documentation

This directory owns both documentation builds:

- `docs/out`: the static Fumadocs product and user documentation, including the embedded ReDoc API reference at `/reference/api/`;
- `docs/api-out`: the independently deployable, read-only ReDoc API Reference.

They share one npm package and lockfile, but remain separate static outputs and
can be hosted on different domains.

The site root displays the documentation introduction inside the standard sidebar
and search layout. Canonical pages use root paths such as `/getting-started/install/`.
The `/docs/...` paths export redirect pages with canonical destinations,
`noindex`, and fallback links. Redirects preserve queries and fragments and apply
the section mappings in `redirects.json`. The **Citadel Docs** brand returns to `/`;
**Website** opens the main portal at `https://citadelplane.com/`.
The `/#licensing` link opens `/overview/licensing/`, preserving query parameters.

## Local development

```bash
npm ci --prefix docs
npm run dev --prefix docs
```

Local development defaults to `http://localhost:3000`. Set explicit URLs when
checking the static exports locally:

```text
NEXT_PUBLIC_DOCS_URL=http://localhost:3000
```

## Validation

```bash
npm run test:config --prefix docs
npm run validate --prefix docs
npm run lint --prefix docs
npm run types:check --prefix docs
npm run build --prefix docs
npm run test:static --prefix docs
```

Build and validate the API Reference:

```bash
npm run build:api --prefix docs
npm run test:api --prefix docs
npm run start:api --prefix docs
```

`build:api` validates and renders the committed `schema/public-v1.json` generated
by Rust. After changing API contracts, run `cargo run --locked -p xtask -- openapi`
from the repository root to regenerate both schemas and the frontend client. Do not copy or edit that schema by hand. The API output
contains a downloadable copy named `public-openapi.json`.

The main documentation site renders the same public schema directly with ReDoc.
Its `/api/openapi` route is exported as a static JSON file, so both local
development and static hosting work without a running Citadel backend or a
separate API documentation host. The renderer is loaded only on the API page
and follows the documentation site's light or dark theme. ReDoc navigation uses
hash routes so endpoint links work on static hosting, including after a reload.
ReDoc's own telemetry is disabled. The API page uses
a full-width layout with ReDoc's endpoint navigation and a compact Citadel
header; **Back to docs** returns to the regular documentation layout.

Edit pages under `content/docs` and update the matching `meta.json` when
navigation changes.

## Visual theme

The product documentation and embedded API reference use the original documentation
backgrounds: light slate `#f8fafc` canvas, white panels, and neutral borders.
Cyan `#006194` accents and square corners follow the Citadel portal.
Dark mode uses slate surfaces and blue `#60a5fa` accents.
`src/app/palette.css` owns the local palette so the documentation builds
independently, with interaction accents aligned with `Citadel.Web/src/frontend/src/layout/palette.css`;
`global.css` maps it to Fumadocs semantic colors, while
`api-reference.css` maps it to ReDoc's CSS variables. Keep both mappings aligned when updating
the palette. Warning/success/error callouts and code syntax colors retain their
semantic meanings. The separate standalone API build keeps its own ReDoc defaults.

Typography uses the original Inter/system font stack without downloading fonts.
Fumadocs provides the default sizing and reading width: 16px article text,
14px navigation, and 28px page titles. The README and documentation use
the blue Citadel logo, matching the favicon artwork.
Navigation, search and theme persistence remain Fumadocs-owned.
The docs sidebar uses `src/app/sidebar.css` for compact navigation rows and a
restrained active state. Search stays above the navigation tree; utility links,
GitHub and theme controls sit in the sidebar footer. Mobile screens use a compact
bar for search and navigation. Touch devices retain larger navigation targets.
The desktop navigation stays at the viewport's left edge, with the table of
contents at the right edge. The article is centered between them and keeps a
bounded reading width.

`scrollbars.css` follows Citadel's frontend scrollbar styling: 4px tracks in
WebKit/Chromium, native thin tracks in Firefox, and subdued thumbs that brighten
on hover. Sidebar thumbs appear on hover or keyboard focus on desktop and stay
available on touch devices, using Fumadocs' existing scroll-area behavior.

## Static export configuration

### Optional documentation analytics

The production docs use a separate **Citadel Docs** website in the existing
self-hosted Umami instance. `src/components/analytics` owns basic/full/off choices and
best-effort page views and explicit interaction allowlists. The root layout provides an allowlist of page paths and
titles from the documentation source; redirects, unknown paths, search text,
queries, fragments and referrers are not collected. The embedded API reference
is included; the separate `api-out` export has no analytics integration.

Collection requires both the build URL and browser origin to be
`https://docs.citadelplane.com`. The browser fetches the public runtime config at
`https://analytics.citadelplane.com/docs/config.json`, then sends permitted page
views and events to `/docs/api/send` on that host. Missing, disabled or invalid configuration
fails closed. No website ID, secret or additional GitHub Actions variable is
needed in the build. Existing `DOCS_SITE_URL` publication configuration suffices.

Nothing is collected before a choice. **Reject non-essentials enables basic
measurement** of source-listed public articles (excluding `/privacy`), plus only
`page_helpful`, `page_unhelpful`, `install_command_copied` and
`openapi_download_requested` on eligible pages. Both the page and event allowlists
are enforced at the send boundary. Basic events never contain a `data` object.
**Accept all** retains all source-listed pages and the full event set below,
including its validated properties. Search remains full-only.

**Preferences** and **Analytics preferences** open a centered dialog with Basic,
Full and **Off** choices. Basic is preselected when no choice is saved; collection
remains disabled until the visitor confirms. Changes apply on **Save preferences**;
closing or pressing Escape discards unsaved changes and restores focus to the trigger.
Docs preferences remain independent of the website, with 180-day expiry,
cross-tab changes and DNT blocking both levels. GPC blocks Basic; an explicit
Full choice takes precedence while valid, including after reload. Expiry disables collection.
Changing between basic and full cancels pending requests without recounting the
current page; navigating away and back counts a new pageview.
Requests omit credentials and referrers, time out after four
seconds and are never retried. See the [visitor privacy page](content/docs/privacy.md).

The analytics provider wraps Fumadocs' root provider so its search dialog shares
consent with page views. Search uses the existing dialog close/select callbacks;
no document-wide click scraping or search-query telemetry is involved.

| Event | Meaning and extra fields |
| --- | --- |
| `search_used` | Final settled nonempty search when the dialog closes or a result is selected; `result_count` is the number of displayed result entries, capped at 1000. |
| `search_no_results` | That completed search returned zero entries. |
| `search_result_opened` | A result was selected by pointer or keyboard; `selected_page` is an allowlisted page path with no query or hash. |
| `page_helpful` / `page_unhelpful` | Yes/No feedback, attributed to the current page; no additional data. |
| `install_command_copied` | Successful clipboard write from a marked install/quick-start example; fixed `step`: `download`, `start`, or `start_tls`. No command text. |
| `openapi_download_requested` | The embedded API reference's schema download link was selected; no additional data or completion claim. |

Search events are attributed to the page where search was used. Intermediate
keystrokes, empty queries, pending requests and search failures are not counted.
A search interaction records its final query's outcome once, so counts do not
represent every query tried. Helpfulness appears only when the chosen level
permits the current page and no browser privacy signal blocks collection.
Responses are deduplicated per page in memory until reload; no vote history is
persisted. Delivery is best-effort, without retries or acknowledgement guarantees.

Use `search_no_results / search_used` to assess completed searches with no hits,
compare `search_result_opened` against `search_used` for result selection, and
review `page_unhelpful` alongside page views and total feedback by page. These
are counts from measured readers, not unique users or proof of successful
installation. Search terms cannot be reconstructed from this instrumentation;
use issues and support conversations to understand missing topics.

The collector, Umami registration and scoped 90-day retention are operated by
the sibling **Citadel.Web** repository's `scripts/analytics/add-docs.py`; its
`docs/ANALYTICS.md` describes installation and rollback. Deploy that configuration
before publishing this docs build through the existing release workflow.

The shared Umami stack uses monthly salt rotation to estimate returning readers
within each UTC calendar month. Umami derives its pseudonymous session ID from
the website ID, IP address, user agent and rotating salt; no browser visitor ID
or account identity is added. These clients do not retain Umami's cache token,
so separate visit IDs use hourly grouping rather than exact browser sessions.
Docs and portal visitors remain separate. The Retention report covers permitted,
unblocked traffic; IP/browser changes and
month boundaries break recognition. Existing installations need the explicit
salt-setting rollout in `Citadel.Web/docs/ANALYTICS.md`; historical daily IDs are
not retroactively joined.

`Citadel.Web/scripts/analytics/configure-audience.py` creates private native Umami
boards for the docs and portal. The docs board includes traffic, entry/exit pages,
installation-copy intent, completed searches without results, and helpfulness by
page. Its page tables count measured readers, while its event table counts
interactions. Traffic and feedback combine basic and full measurement; search
reports cover only full mode. Neither represents readers who disabled analytics.
Use the separate Retention report for return visits. Existing board
customizations survive reruns.

Browser regression checks intercept all collector requests against a local
export and never submit production analytics:

```bash
npm ci --prefix test/e2e
npm exec --prefix test/e2e -- playwright install --with-deps chromium
NEXT_PUBLIC_DOCS_URL=https://docs.citadelplane.com npm run build --prefix docs
npm run test:analytics --prefix docs
```

They cover basic/full/off choices, expiry, DNT/GPC, storage failures, clean payloads,
navigation deduplication, cross-tab changes, collector failures, unknown/local pages, mobile controls,
search outcomes, keyboard selection, query exclusion and page feedback.

### Publication URL

The product site's build URL can include a path, for example
`http://localhost:3000/Citadel`. Set `NEXT_PUBLIC_DOCS_URL` before building; the
build derives Next.js `basePath` from that path and prefixes images, search and
navigation accordingly. Rebuild after changing the URL, then run `test:static`
to check the generated paths. Ordinary CI checks use local URLs so contributors
can validate exports without setting up public hosting.

Stable and numbered development release tags publish the validated static export
to the same GitHub Pages site. The site follows the highest published version across
both channels; see [Continuous integration](CI.md#release-documentation-publication)
for publication and retry behavior.

The standalone API output uses relative asset paths. The product site's
**API Reference** navigation link opens its embedded reference.
Run `npm run start:api --prefix docs -- --listen 3001` to serve the API export
at the example address above. See [Continuous integration](CI.md) for checks.

Keep user pages task-focused: explain prerequisites, give the shortest useful
steps, show what success looks like, and put detailed controls after that path.
Use current button and menu labels. Reuse real screenshots from `public/screenshots`
with descriptive alt text and say when they show demonstration data.

## Guide screenshots

Use screenshots for configuration decisions and useful result states. Keep simple
navigation and single-field instructions as text. Crop to the relevant panel,
use demonstration resources, add descriptive alt text and a short explanation,
and keep all essential instructions in the text. Use standard Markdown images;
the docs renderer adds click-to-zoom automatically. Readers can also focus the
zoom control and press Enter, then close it with Escape or the close control.
Do not wrap screenshots in links to their original files.

Every saved PNG must be at least 1660 pixels wide. Capture at a 1660px browser
viewport; guide crops use 4x pixel density so narrow dialogs meet the minimum
without enlarging a bitmap. README captures use 1x density. The shared screenshot
writer checks the PNG width after each capture and fails if it is too small.

To refresh the Git Stack, Swarm coverage, backup policy, scripts, and Role
screenshots, including restore selection, resource access, deployment failures,
Email/Discord notification dialogs, network map, container health details,
and volume browser, start the frontend development server and run from the repository root:

```bash
npm ci --prefix test/e2e
cd test/e2e
npx playwright install chromium
cd ../..
node docs/scripts/capture-guide-screenshots.mjs
```

Refresh the README images, including light/dark platform views,
statistics, and the network map, with:

```bash
node docs/scripts/capture-guide-screenshots.mjs readme
```

To capture only the restore, resource-access, and failure examples:

```bash
node docs/scripts/capture-guide-screenshots.mjs restore access failure
```

To capture only the network map, health details, and volume browser:

```bash
node docs/scripts/capture-guide-screenshots.mjs network health volume
```

Set `CITADEL_SCREENSHOT_BASE_URL` if the frontend uses a different address.
The script uses a fresh browser context and intercepted API fixtures; no login,
running backend, or real resources are needed. README and guide screenshots use
the Blue accent, including the light and dark README variants. Writes and
unknown API requests are blocked, and realtime messages are handled locally.
A fixed browser clock keeps example health observations fresh and timestamps reproducible.
On Linux, install Chromium's system dependencies with
`npx playwright install --with-deps chromium` from `test/e2e` if needed.
The permission matrix fixture contains public capability metadata,
not user assignments. Refresh it when the supported permissions change.

After capture, inspect the PNGs in `docs/public/screenshots`, check the surrounding
captions, and run the documentation validation. Check the affected pages at both
desktop and mobile widths. These captures document example configurations, not
successful operations against a live Docker environment.
