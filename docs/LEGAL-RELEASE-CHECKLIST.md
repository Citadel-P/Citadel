# Legal distribution checklist

Maintainer checks for Citadel distributions under Elastic License 2.0 (ELv2).
This is not a legal opinion, complete dependency audit or compliance certificate.

## Ownership and licensing authority

- [ ] Confirm the copyright holders and authority to offer Citadel-authored
  material under ELv2, including migrated .NET code, contractor contributions,
  generated code, documentation, images and other assets. Resolve employer
  ownership and third-party licensing questions before distribution.
- [ ] Verify dependency compatibility and retain separate third-party grants.
  A root ELv2 license does not relicense externally owned material.
- [ ] Establish and securely record any necessary contributor permissions
  before accepting code for separately negotiated commercial distributions.
  No approved CLA or automated signature workflow is active yet.
- [ ] Confirm rights to project names and logos before relying on
  [TRADEMARKS.md](../TRADEMARKS.md) for enforcement.

## Earlier public licensing snapshots

Earlier public PR #47 revisions contained an AGPL-3.0-only proposal, including
`d476d34f5ec9d9d439aae47d933ee8420458e87f` and
`ae38ab491c9ddd936936d8b1631791d8e13bf0f9`.

- [ ] Have counsel determine the licensing effect of those published snapshots
  and identify any rights already granted for copies of that code. A draft PR
  is not necessarily a private or legally ineffective publication.
- [ ] Do not claim this update, a merge, branch deletion or history rewrite
  revokes valid earlier grants. Preserve provenance and historical notices.
- [ ] Establish which material can be offered under ELv2 and what previous
  rights remain before making exclusivity or enforcement claims.

This revision uses ELv2; it does not revoke rights already granted for earlier
copies. The scope and effect of any previous grants require legal review.

## Before each source or binary distribution

- [ ] Keep the canonical, unmodified ELv2 text in root LICENSE and include
  `/app/LICENSE` in both Core and Agent images. Use `Elastic-2.0` for their
  Citadel source-license metadata; bundled components retain their own terms.
- [ ] Ensure recipients receive the license terms with copies of the software.
  Retain licensing, copyright and other notices, and prominent notices for
  modifications. Apply the ELv2 license-key and protected-functionality terms.
- [ ] Describe the product as source-available, not OSI-approved open source.
  Distinguish the source license from signed Team product entitlements.
- [ ] Verify the sign-in/setup and account-menu license and source links.
  Commit-specific source links are retained for transparency and provenance;
  ELv2 itself does not impose AGPL's network-source-disclosure obligation.
  Third-party source-disclosure obligations, where applicable, still apply.
- [ ] Inventory licenses and required notices for Rust/npm dependencies,
  generated code, fonts, assets, Docker tools, Deno and OS/runtime packages.
  Provide source or other materials where their own licenses require it.
- [ ] Verify the known Unicode-3.0 time-zone and MIT password-data notices
  remain under `/app/third-party-notices/`. These two notices are not a
  complete dependency inventory or evidence of full compliance.
- [ ] Inspect built images and retain a release-specific license inventory or
  SBOM. Source-tree recipe tests alone cannot verify final distributions.

## Product and partner agreements

Community remains free for personal and internal business use under ELv2;
Team keys enable paid capabilities. Neither a Team key nor an explanatory
Markdown file grants hosted-service, reseller or alternative source rights.

- [ ] Publish reviewed subscription/order terms covering entitlements,
  duration, renewal and support before taking paid orders.
- [ ] Review consultant/MSP arrangements according to actual access to Citadel
  functionality, rather than imposing a blanket MSP ban. Grant any required
  hosting/reseller permission through an executed written agreement.

Keep [the licensing guide](content/docs/overview/licensing.md),
[commercial plan explanation](../COMMERCIAL-LICENSING.md) and
[contribution policy](../CONTRIBUTING.md) consistent. The baseline tests in
`test/release/test_licensing.py` do not substitute for these human reviews.
