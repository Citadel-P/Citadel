# Citadel documentation

This directory owns both documentation builds:

- `docs/out`: the static Fumadocs product and user documentation;
- `docs/api-out`: the independently deployable, read-only ReDoc API Preview.

They share one npm package and lockfile, but remain separate static outputs and
can be hosted on different domains.

## Local development

```bash
npm ci --prefix docs
npm run dev --prefix docs
```

Local development defaults to `http://localhost:3000`. Set explicit URLs when
checking an export locally or preparing a publication:

```text
NEXT_PUBLIC_DOCS_URL=https://docs.example.com
NEXT_PUBLIC_API_DOCS_URL=https://api-docs.example.com
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

Build and validate the API Preview:

```bash
npm run build:api --prefix docs
npm run test:api --prefix docs
npm run start:api --prefix docs
```

`build:api` validates and renders the committed `schema/public-v1.json` generated
by Rust. After changing API contracts, run `cargo run --locked -p xtask -- openapi`
from the repository root to regenerate both schemas and the frontend client. Do not copy or edit that schema by hand. The API output
contains a downloadable copy named `public-openapi.json`.

Edit pages under `content/docs`. Keep `docs-audit.md` outside that directory so
it remains an implementation record rather than a public page. Update the
matching `meta.json` when navigation changes.

Documentation published as stable must be built from a stable Citadel tag, not
from an arbitrary `main` commit.

Until a separate API-reference hostname and deployment target are configured,
the release workflow publishes `docs/api-out` as an independent artifact.

## GitHub Pages and CI

`DOCS_SITE_URL` and `API_DOCS_URL` are GitHub repository variables. Release builds
require both. Ordinary documentation checks use local test URLs when the variables
are absent, so pull requests and forks can validate the docs before hosting exists.

The product site's URL can include a path, for example
`https://citadel-p.github.io/Citadel`. The build derives Next.js `basePath` from
that path and prefixes images, search, and navigation accordingly. A custom
root domain does not need a prefix. Set the URL before building; changing it
requires rebuilding the static output.

The API output uses relative asset paths and can be hosted independently. Setting
`API_DOCS_URL` adds its navigation link; it does not deploy that artifact.
See [CI and release configuration](CI.md) for the publication settings and checks.

Keep user pages task-focused: explain prerequisites, give the shortest useful
steps, show what success looks like, and put detailed controls after that path.
Use current button and menu labels. Reuse real screenshots from `public/screenshots`
with descriptive alt text and say when they show demonstration data.

## Guide screenshots

Use screenshots for configuration decisions and useful result states. Keep simple
navigation and single-field instructions as text. Crop to the relevant panel,
use demonstration resources, add descriptive alt text and a short explanation,
and keep all essential instructions in the text. Link images to their original
files so readers can enlarge them.

To refresh the Git Stack, Swarm coverage, backup policy, automation, and Role
screenshots, including restore selection, resource access, and deployment failures,
start the frontend development server and run from the repository root:

```bash
npm ci --prefix test/e2e
cd test/e2e
npx playwright install chromium
cd ../..
node docs/scripts/capture-guide-screenshots.mjs
```

To capture only the restore, resource-access, and failure examples:

```bash
node docs/scripts/capture-guide-screenshots.mjs restore access failure
```

Set `CITADEL_SCREENSHOT_BASE_URL` if the frontend uses a different address.
The script uses a fresh browser context and intercepted API fixtures; no login,
running backend, or real resources are needed. Writes and unknown API requests
are blocked. The permission matrix fixture contains public capability metadata,
not user assignments. Refresh it when the supported permissions change.

After capture, inspect the PNGs in `docs/public/screenshots`, check the surrounding
captions, and run the documentation validation. Check the affected pages at both
desktop and mobile widths. These captures document example configurations, not
successful operations against a live Docker environment.
