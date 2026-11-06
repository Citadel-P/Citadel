---
name: citadel-release-workflow
description: Prepare, publish, verify, or recover a Citadel product release using its version resolver and GitHub Actions workflow, including release-note commit audits and release-driven demo or preview deployment. Use for release tags, commit-title repairs for release notes, publication failures, and deployment verification; not for ordinary feature edits or instance upgrade implementation.
---

# Citadel release workflow

Work from the repository root. Follow `AGENTS.md` and the user's requested scope.
Preparing or reviewing a release does not itself authorize publication or deployment.
When those actions are already authorized, complete them without adding another
approval step. Never infer permission to replace an existing tag or discard evidence.

## Establish the release identity

Read the relevant sections of `docs/DEVELOPMENT.md` (Build version metadata) and
`docs/release-publication.md`. Inspect `.github/workflows/ci.yml` and
`src/tools/build/version.py` for the current eligibility rules; use the resolver
instead of reimplementing version arithmetic or guessing tag names.

Inspect the working tree, intended source commit, remote main and existing tags.
Determine the requested stable/development channel and deployment target from the
task and repository evidence. Ask only when a missing choice affects the release.
Do not silently include unrelated working changes or tag a different commit.
Use an isolated checkout if the working tree must remain undisturbed.

Use complete Git history and the pinned version tooling. `version.json` owns the
product version; commit an authorized version change before resolving release
identity. Release tags must be annotated and match the intended commit and resolved
version. Inspect conflicting local/remote tags rather than force-moving them.
Keep temporary metadata, notes and downloaded evidence outside the checkout.

## Audit commit titles for release notes

Use `src/tools/release/cliff.toml` and `src/tools/release/notes.py` as the source of
truth for classification. Distinguish malformed Conventional Commit titles from
valid custom types that Citadel does not classify. Ordinary `docs`, `ci`, `build`,
`chore`, `deps`, `test`, and `style` commits are intentionally filtered; do not
relabel them as fixes merely to include them in release notes. Choose types from
the actual change, preserving bodies, breaking markers/footers and PR references.
For squash merges, check the final squash title before merging; changing a merged
PR title does not update the existing commit.

For a history repair, inspect the exact remote head, reachable release tags and
affected descendants first. The notes generator selects predecessors through tag
ancestry: rewriting a published ancestor can make later releases include the full
history. Prefer unreleased corrections when that satisfies the request. If the
requested scope crosses published tags, explain this consequence and resolve the
scope before pushing; do not move release tags or alter publication records as an
incidental repair. An audit alone does not authorize rewriting remote history.

Prepare an authorized rewrite with a recoverable original ref and an old-to-new
commit map outside the working tree. Verify each rewritten commit's tree, parent
topology, author/date and message body; retain valid titles and strip invalidated
signatures rather than copying them to changed commits. Check that the final tree
is identical and release predecessors remain correct, then preview classification
with the pinned git-cliff. Push only the intended branch using an explicit lease
against the inspected remote SHA. If the lease or branch rules reject the push,
inspect the cause; do not bypass repository rules or retry with an unconditional
force. Report the corrected titles, backup location, remaining historical issues,
and changed commit IDs/signature status.

## Prepare and publish through the existing workflow

Use `src/tools/release/notes.py` to preview notes from the exact release revision.
Inspect its review output for missing or misleading entries. Follow the publication
guide and existing tooling for prerequisites, acceptance and provenance; do not
create a parallel sequence of manual registry pushes or GitHub release uploads.

For an authorized publication, push the intended commit/tag through the existing
workflow and follow the run associated with that exact revision. A manual workflow
dispatch is not a substitute for the validated tag-push publication context.
Do not manufacture GitHub event variables to bypass the publisher's guards.

Distinguish candidate builds, acceptance, publication and environment deployment.
Report completion only after checking the relevant job outcomes and public release
record, including the manifest and checksums. Signature receipts are not independent
signature verification; use the documented verifier when that verification is needed.
Preserve the workflow's coordinated Core/Agent, registry and documentation gates.

## Verify demo or preview deployment when requested

Inspect the deployment jobs in `.github/workflows/ci.yml` and
`.github/actions/deploy-demo/action.yml`. Currently stable releases target demo
through the `latest` channel; development releases target preview through `dev`.
Check each job's enable flag and environment configuration instead of assuming a
successful publication means deployment ran.

Follow the deployment summary and verify the expected running version and public
health. Report disabled, skipped, stale-alias and failed outcomes distinctly. Reuse
the release manifest's immutable identities and existing deployment tooling;
preserve storage and SSH host verification. Keep credentials out of logs and Git.

## Recover an interrupted run

Read the publication guide's retention/retry rules and inspect the original run's
logs, reservation and retained evidence before choosing recovery commands. Use the
original workflow rerun when applicable. Do not rebuild accepted artifacts, replace
product tags, clobber attachments or delete incomplete recovery drafts to clear a
failure. If required evidence is missing or conflicting, stop the dependent mutation
and report the specific evidence needed to proceed.

For completed-publication cleanup, use the documented dry run and exact resolved
metadata. Apply cleanup within the user's authorized scope only after inspecting its
result. Publication recovery and installation/data recovery are separate workflows.

In the final response, identify the commit, tag/version, run/release links and any
requested environment's verified outcome. State pending or unverified stages. If
release tooling itself changed, run its affected tests from `test/README.md`; never
weaken publication guards to obtain a passing run.
