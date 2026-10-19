# Citadel documentation

This directory owns both documentation builds:

- `docs/out`: the static Fumadocs product and user documentation, including the embedded ReDoc API reference at `/docs/reference/api/`;
- `docs/api-out`: the independently deployable, read-only ReDoc API Reference.

They share one npm package and lockfile, but remain separate static outputs and
can be hosted on different domains.

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
and follows the documentation site's light or dark theme. The API page uses
a full-width layout with ReDoc's endpoint navigation and a compact Citadel
header; **Back to docs** returns to the regular documentation layout.

Edit pages under `content/docs` and update the matching `meta.json` when
navigation changes.

## Visual theme

The product documentation, landing page and embedded API reference use slate
surfaces with blue interaction accents in both light and dark modes. Light-mode
actions use blue `#2563eb`; dark-mode accents use light blue `#93c5fd`.
`src/app/palette.css` owns the local palette so the documentation builds
independently; `global.css` maps it to Fumadocs semantic colors. The landing page
reuses those tokens, while `src/components/api-reference.tsx` supplies matching concrete colors
to ReDoc (which computes derived colors). Keep both mappings aligned when updating
the palette. Warning/success/error callouts and code syntax colors retain their
semantic meanings. The separate standalone API build keeps its own ReDoc defaults.

Typography follows the published documentation's Inter/system fallback stack
without downloading a web font. Article and navigation sizing remain
Fumadocs-owned. The README and documentation use the blue Citadel logo, matching
the favicon artwork.
Reading widths, navigation, search and theme persistence remain Fumadocs-owned.
The docs sidebar uses `src/app/sidebar.css` for compact navigation rows and a
restrained active state. Search stays above the navigation tree; utility links,
GitHub and theme controls sit in the sidebar footer. Mobile screens use a compact
bar for search and navigation. Touch devices retain larger navigation targets.
The desktop navigation stays at the viewport's left edge, with the table of
contents at the right edge. The article is centered between them and keeps a
bounded reading width.

## Static export configuration

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

To refresh the Git Stack, Swarm coverage, backup policy, automation, and Role
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

Refresh the README and homepage images, including light/dark platform views,
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
