# Contributing to Citadel

Bug reports, reproducible issues, documentation improvements, tests and
Community feature fixes are welcome. Discuss substantial changes in a GitHub
issue before implementation.

## Source terms and upstream review

Citadel-authored source in this revision uses the
[Elastic License 2.0](LICENSE) (`Elastic-2.0`). It is source-available, not
OSI-approved open source. Review its permissions and limitations before
modifying or redistributing the software.

Community and Team implementations share Rust crates, APIs and React
components. Directories are not separate license boundaries. The official
project reserves decisions about paid-feature behavior, activation,
license-key verification and licensing infrastructure to maintainers.
Do not submit unsolicited PRs changing those behaviors; open an issue first.
Community fixes in shared files can still be considered.

Upstream review approval is not a license exception. For work that changes
license-key functionality or otherwise needs rights beyond ELv2, obtain
separate written permission from the relevant rights holder before doing
that work. Report security concerns through [SECURITY.md](https://github.com/Citadel-P/Citadel/security/policy)
when a private reporting channel is available; never publish credentials or
private signing keys.

## Contributor permissions

Contributors retain ownership of their work. Maintainers must verify that a
submission can lawfully be included in an ELv2 distribution, including any
employer permission and obligations attached to third-party material.

A separate written contributor license agreement (CLA), or equivalent express
permission, is required before accepting contributions that the project needs
to distribute under separately negotiated commercial terms. ELv2's public
license is not an automatic grant of commercial relicensing authority to the
maintainers. **No approved CLA or electronic signing workflow is active yet.**
External code contributors should coordinate permissions before investing in
a PR, and maintainers must record any required signed agreement before merge.
A PR submission, checkbox or Git sign-off is not an executed CLA or copyright
assignment. Legal review should cover copyright, patent and employer rights.

Documentation corrections and issue reports are welcome; maintainers should
identify any additional permissions needed for contributed text or assets.
This policy does not revoke rights already granted for earlier copies.

## Submitting a change

1. Align on scope in an issue and secure any necessary permissions.
2. Keep the change focused; avoid unrelated refactors and generated-file churn.
3. Follow [development and validation guidance](docs/DEVELOPMENT.md), adding
   relevant tests and documentation.
4. Open a PR describing behavior, validation and compatibility implications.
5. Wait for maintainer review. [CODEOWNERS](.github/CODEOWNERS) routes reviews
   but does not itself enforce approval. A solo maintainer cannot approve
   their own PR; plan branch protection around the available reviewers.

Do not submit secrets, credentials, private keys or third-party work you do
not have permission to contribute. Preserve applicable license, attribution
and modification notices.
