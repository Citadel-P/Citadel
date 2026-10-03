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

Local development defaults to `http://localhost:3000`. Production and CI builds
must set:

```text
NEXT_PUBLIC_DOCS_URL=https://docs.example.com
NEXT_PUBLIC_API_DOCS_URL=https://api-docs.example.com
```

## Validation

```bash
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
