# Contributing to Citadel

Thanks for your interest in improving Citadel. Bug reports, reproducible issues,
documentation improvements, tests, and Community feature fixes are welcome.
Please discuss substantial changes in a GitHub issue before writing code.

## Community and paid features

Citadel Community and Team implementations share Rust crates, services, API
routes, and frontend components. **Directories are not license boundaries.**

The following are **maintainer-controlled in the official Citadel repository**:

- changes that introduce, alter, or unlock paid Team capabilities;
- license verification, signed entitlements, activation, and license-key checks;
- distribution terms or commercial-source licensing infrastructure.

Please do **not submit unsolicited pull requests** changing those behaviors.
Open an issue instead. Community bug fixes that happen to touch a shared file
may still be considered with maintainer review.

This is an **upstream merge policy, not a restriction on AGPL users**. Under
AGPL-3.0, anyone may modify and redistribute their own forks in compliance
with the license, including changes to paid-feature checks. This project
cannot revoke those permissions by contribution guidelines or CODEOWNERS.

## Contributor licensing

The public source is offered under [AGPL-3.0-only](LICENSE), and Citadel may
also offer separately negotiated commercial source licenses. Contributors
retain ownership of their work. The project must obtain sufficient permission
from contributors **before merging** changes that may be commercially
relicensed.

A separate written contributor license agreement (CLA), or equivalent explicit
permission granting the necessary commercial relicensing rights, is required
before third-party code contributions are merged into code intended for dual
licensing. **No approved CLA text or electronic signature workflow is active
yet.** Please open an issue to coordinate *before* investing in a code PR.
Merely opening a PR, ticking a checkbox, or signing off a commit is **not**
an executed CLA. Maintainers must verify and securely record signed permission
before merging covered contributions. Counsel should review the final agreement,
especially corporate-employer ownership and patent grants.

Documentation fixes, issue reports, and feedback are welcome; maintainers
will determine whether a proposed text contribution needs an additional
rights agreement. Contributions accepted under AGPL alone cannot automatically
be sublicensed under separate commercial terms by Citadel.

## Submitting a change

1. Open an issue or check an existing issue to align on scope.
2. Keep the change focused; avoid unrelated refactors and generated-file churn.
3. Follow [development and validation guidance](docs/DEVELOPMENT.md), and
   include relevant tests and documentation changes.
4. Open a PR describing behavior, tests, and any security or compatibility
   implications.
5. Wait for maintainer review. [CODEOWNERS](.github/CODEOWNERS) routes
   reviews but does not itself enforce approval. A single maintainer cannot
   approve their own PR; only enable mandatory code-owner review after a
   second eligible reviewer is available.

Do not submit secrets, private keys, credentials, or third-party code that
you do not have permission to contribute.
