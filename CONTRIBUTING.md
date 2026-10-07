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
for third-party code contributions. **A CLA signing workflow is not yet
available**: please open an issue to coordinate before proposing code. Merely
submitting a PR or signing off a commit does not grant the project additional
commercial relicensing rights. Maintainers should not merge contributions
requiring those rights without securing that agreement first.

## Submitting a change

1. Open an issue or check an existing issue to align on scope.
2. Keep the change focused; avoid unrelated refactors and generated-file churn.
3. Follow [development and validation guidance](docs/DEVELOPMENT.md), and
   include relevant tests and documentation changes.
4. Open a PR describing behavior, tests, and any security or compatibility
   implications.
5. Wait for maintainer review. Some paths are marked in
   [.github/CODEOWNERS](.github/CODEOWNERS); required review is enforced only
   if the corresponding GitHub branch rules are enabled.

Do not submit secrets, private keys, credentials, or third-party code that
you do not have permission to contribute.
