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
checking the static exports locally:

```text
NEXT_PUBLIC_DOCS_URL=http://localhost:3000
NEXT_PUBLIC_API_DOCS_URL=http://localhost:3001
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

Edit pages under `content/docs` and update the matching `meta.json` when
navigation changes.

## Static export configuration

The product site's build URL can include a path, for example
`http://localhost:3000/Citadel`. Set `NEXT_PUBLIC_DOCS_URL` before building; the
build derives Next.js `basePath` from that path and prefixes images, search and
navigation accordingly. Rebuild after changing the URL, then run `test:static`
to check the generated paths. Ordinary CI checks use local URLs so contributors
can validate exports without setting up public hosting.

The API output uses relative asset paths. Set `NEXT_PUBLIC_API_DOCS_URL` when
testing its navigation link in the product site; leave it unset to omit the link.
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
