# Citadel documentation

This package builds two static outputs: `docs/out` for product documentation and
its embedded API reference, and `docs/api-out` for a standalone ReDoc reference.
They share dependencies but can be hosted separately. Edit pages in `content/docs`
and the matching `meta.json` when navigation changes.

Contributor references: [Architecture](ARCHITECTURE.md),
[Development](DEVELOPMENT.md), [Tests](../test/README.md),
[Agent protocol](agent-protocol.md), [CI](CI.md), and
[release publication](release-publication.md). Keep these references about current
behavior and repeatable procedures. Put task progress, screenshots of completed
reviews and benchmark captures in the change review or external investigation
artifacts, not permanent guides.

## Local development

```bash
npm ci --prefix docs
npm run dev --prefix docs
```

The site defaults to <http://localhost:3000>. Set `NEXT_PUBLIC_DOCS_URL` explicitly
when checking static exports; its path becomes Next.js `basePath`.

## Validation

Run from the repository root. Type generation and builds both write `docs/.next`,
so run them sequentially:

```bash
npm run test:config --prefix docs
npm run validate --prefix docs
npm run lint --prefix docs
npm run types:check --prefix docs
npm run build --prefix docs
npm run test:static --prefix docs
npm run build:api --prefix docs
npm run test:api --prefix docs
```

`build:api` renders the committed Rust-generated `schema/public-v1.json`; never edit
or copy that schema manually. Follow [API generation](DEVELOPMENT.md#api-changes)
after contract changes. The standalone export includes `public-openapi.json` and
can be served with `npm run start:api --prefix docs -- --listen 3001`.

The embedded reference at `/reference/api/` renders the same public schema and
exports `/api/openapi` as static JSON. It needs no running Core or separate API
host. Hash navigation supports static hosting; ReDoc telemetry is disabled.

## Visual theme

`src/app/palette.css` owns local colors; `global.css` maps them to Fumadocs and
`api-reference.css` to ReDoc. Keep both mappings aligned, preserve semantic
warning/success/error colors, and use the existing Inter/system font stack and
blue Citadel artwork. The standalone API export retains its own ReDoc defaults.

Use shared navigation, search, code blocks and theme controls. Keep user pages
task-focused: prerequisites, short steps, expected results, then detailed controls.
Use current UI labels and descriptive screenshot captions.

## Static export configuration

### Optional documentation analytics

`src/components/analytics` owns Basic/Full/Off preferences and explicit event
allowlists. Collection requires both the build URL and browser origin to be
`https://docs.citadelplane.com`. Runtime configuration comes from
`https://analytics.citadelplane.com/docs/config.json`; permitted events go to that
host's `/docs/api/send`. Missing, invalid or disabled configuration fails closed.
No analytics secret or website ID belongs in build variables. The standalone
`api-out` export has no analytics integration.

Nothing is collected before a confirmed choice. **Reject non-essentials** selects
Basic: source-listed public articles except `/privacy`, and the four non-search
events below without a `data` object. **Accept all** selects Full, including
allowlisted properties and search events. Off disables collection. Preferences
expire after 180 days, synchronize across tabs and remain separate from the portal.
DNT blocks collection; an explicit valid Basic/Full choice takes precedence over
GPC. Unsaved dialog changes do not apply.

Never collect search text, queries, fragments, referrers, unknown paths or redirect
pages. Requests omit credentials/referrers, time out after four seconds and are
not retried. Changing consent cancels pending requests without recounting the
current page. See [visitor privacy](content/docs/privacy.md).

| Event | Permitted meaning/properties in Full mode |
| --- | --- |
| `search_used` | Final settled nonempty search on close/selection; displayed `result_count`, capped at 1000 |
| `search_no_results` | Completed search with zero entries |
| `search_result_opened` | Pointer/keyboard selection; allowlisted `selected_page` without query/hash |
| `page_helpful` / `page_unhelpful` | Current-page feedback, no extra data |
| `install_command_copied` | Successful copy; fixed `step`: `download`, `start`, `start_tls`; no command text |
| `openapi_download_requested` | Schema link selected, not proof of completed download |

Intermediate keystrokes, pending/failed searches and empty queries are excluded.
Feedback deduplicates per page in memory until reload. These are best-effort
interaction counts, not unique-user or successful-installation counts.

The sibling Citadel.Web repository owns the collector, 90-day retention, Umami
registration and dashboards: see its `docs/ANALYTICS.md`,
`scripts/analytics/add-docs.py` and `scripts/analytics/configure-audience.py`.
Monthly salt rotation and hourly visit grouping limit return-visitor recognition;
portal/docs identities remain separate. Follow that repository's rollout/recovery
instructions for infrastructure changes.

Analytics browser checks intercept collector requests and send no production data:

```bash
npm ci --prefix test/e2e
npm exec --prefix test/e2e -- playwright install --with-deps chromium
NEXT_PUBLIC_DOCS_URL=https://docs.citadelplane.com npm run build --prefix docs
npm run test:analytics --prefix docs
```

### Publication URL

Rebuild after changing `NEXT_PUBLIC_DOCS_URL`. To test a Pages subpath, use
`http://localhost:3000/Citadel` and run `test:static`. Standalone API assets use
relative paths. CI validation uses local URLs; release publication uses
`DOCS_SITE_URL` and follows the highest published SemVer across stable/development
channels. See [CI publication](CI.md#release-documentation-publication).

Canonical product pages use root paths such as `/getting-started/install/`.
`/docs/...` redirect pages use `redirects.json`, preserve queries/fragments and
provide canonical/noindex metadata and fallback links. The brand links to `/`;
Website links to the portal. `/#licensing` redirects to `/overview/licensing/`.

## Guide screenshots

Capture configuration decisions and useful result states, using demonstration
resources. Crop to the relevant panel, include alt text/captions, and retain all
essential instructions in text. Standard Markdown images receive accessible zoom;
do not wrap them in links to the original files.

Every PNG must be at least 1660 pixels wide. Use a 1660px viewport, 4x density for
narrow guide crops, and 1x for README images. The screenshot writer enforces width;
do not upscale small bitmaps.

Start the frontend development server, then run from the repository root:

```bash
npm ci --prefix test/e2e
npm exec --prefix test/e2e -- playwright install --with-deps chromium
node docs/scripts/capture-guide-screenshots.mjs
# README captures or selected guide groups:
node docs/scripts/capture-guide-screenshots.mjs readme
node docs/scripts/capture-guide-screenshots.mjs restore access failure
node docs/scripts/capture-guide-screenshots.mjs network health volume
```

Set `CITADEL_SCREENSHOT_BASE_URL` for a different frontend address. The runner uses
a fresh browser context, fixed clock, Blue accent and intercepted API/realtime
fixtures; no backend or login is needed. Writes and unknown API requests are
blocked. Refresh the public capability fixture when permissions change.

Inspect the resulting `docs/public/screenshots` images and captions at desktop
and mobile widths, then validate documentation. These captures demonstrate UI
configuration, not successful operations against a live Docker environment.
