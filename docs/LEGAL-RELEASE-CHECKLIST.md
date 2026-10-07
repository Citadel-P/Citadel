# Legal distribution checklist

A maintainer checklist for the first AGPL release and later Core/Agent images.
It is **not** a legal compliance certification or legal advice.

## Before the first AGPL release

- [ ] Verify the actual copyright holders and authority to license Citadel
  source, including migrated .NET code, contractor work, generated material,
  documentation, frontend assets and logos. Resolve any employer ownership.
- [ ] Approve contributor relicensing terms with counsel and arrange for
  verifiable signing/storage before merging third-party code that may appear
  in commercial source distributions. The project has no approved CLA or
  automated signing workflow yet.
- [ ] Confirm who owns rights to the project name and logos before relying
  on the [trademark policy](../TRADEMARKS.md) for enforcement.

## Before every official distribution

- [ ] Include the root `LICENSE` in both Core and Agent images and set the
  OCI license label to `AGPL-3.0-only`.
- [ ] Confirm source links from unauthenticated setup/login screens and the
  signed-in account menu. They must point to the **Corresponding Source**
  for the exact deployed revision, including the relevant source, build
  scripts, generated-code inputs and local modifications.
- [ ] Ensure legal notice text discloses AGPL redistribution rights and lack
  of warranty. Retain the notices in modified distributions, as required.
- [ ] For downstream forks, update the frontend build argument
  `VITE_CITADEL_SOURCE_REPOSITORY_URL` to the fork's repository, preserve
  the full build `SOURCE_REVISION`, and publish the full modified
  Corresponding Source. A link to unmodified Citadel is not sufficient.
  If the exact revision is unavailable, provide an equivalent accurate
  source-download link instead of assuming upstream's main branch matches.
- [ ] Inventory and review applicable licenses and attribution obligations
  for all Rust and npm dependencies, generated API code, assets, fonts, and
  binary/runtime software included in images (e.g. Docker CLI, Deno and OS
  packages). Retain required notices; the root AGPL license does not
  override their terms.
- [ ] Confirm the known Unicode-3.0 time-zone and MIT common-password
  notices remain in the image at `/app/third-party-notices/`.
  **Those two notices are not a comprehensive third-party inventory.**
- [ ] Record a release-specific dependency/license inventory or SBOM and
  obtain legal sign-off when required.

`test/release/test_licensing.py` performs limited source-tree/recipe checks.
It does not replace full build inspection, SBOM review, license notices, or
verification of a hosted exact-source offer.

## Commercial subscriptions are separate

The public AGPL source license and Citadel's signed Team product license
grant different rights. An activation key does not grant an alternative
commercial source license, and the public AGPL allows forks that modify
feature checks (subject to AGPL compliance). An alternative commercial
source license needs an executed agreement and suitable rights for all
included contributions and dependencies.

Keep the [licensing guide](content/docs/overview/licensing.md),
[commercial terms explanation](../COMMERCIAL-LICENSING.md), and
[contribution rules](../CONTRIBUTING.md) consistent.
