# Release publication and recovery

The annotated product tag and Nerdbank.GitVersioning resolver remain authoritative.
Agent images are verified and signed before Core promotion. Existing acceptance,
compatibility, registry and Pages gates still apply. Stable images publish to GHCR
and Docker Hub; development images retain the existing GHCR-only policy.

## Notes and public files

`src/tools/release/git-cliff-pin.json` pins the git-cliff version and archive SHA256.
`cliff.toml` groups features, fixes, performance and improvements, filtering ordinary
CI, dependencies, tests, documentation and maintenance. Breaking changes bypass
these filters. Direct, squash and merge commits are supported; matching merge titles
are deduplicated. Nonconventional/unclassified or obviously vague messages appear
in `release-review.json` and the workflow summary for maintainer review. This is a
review warning, not a new manual approval gate. Unclassified changes remain visible
under “Other changes”; explicit breaking footers are preserved even on these commits.

Notes are generated from the exact tagged SHA, never the runner's later HEAD.
Stable releases compare against the highest preceding reachable stable tag.
Development releases compare against the preceding reachable stable/development tag.
Intermediate development tags do not split stable notes. A first release includes
its reachable history. Preview notes before pushing an annotated local release tag:

```bash
python3 src/tools/release/notes.py install
DOCS_SITE_URL=https://docs.citadelplane.com python3 src/tools/release/notes.py generate \
  --metadata /path/to/resolved-version.json --directory /tmp/release-notes
```

The public release has two uploaded files: `release-manifest.json` and `SHA256SUMS`.
Deployment jobs read and verify this public manifest with their existing read-only
tokens; they do not need access to recovery drafts.
The manifest lists immutable image references, verified signature checkpoints, architectures, Agent-before-Core
upgrade order, and the existing cosign certificate identity/issuer. GitHub additionally
supplies its standard source archives. Notes link installation/upgrade documentation
and versioned images. The public draft is exposed only after images, signatures,
aliases, documentation and public attachment checks succeed. Registry copies cannot
be transactional across providers; the existing journal resumes partial publication.
The GitHub release page is the final user-facing completion signal.

## Retention and retries

**Thirty-day Actions retention is not a durable recovery policy.** Actions artifacts
remain 30-day transfer/fallback copies. For new releases, the workflow first reserves
an internal draft named `citadel-recovery/vVERSION`, binding the tagged revision,
compatibility inputs and original Actions run. It never publishes this draft or
creates an internal Git tag. It uses the existing repository and GitHub token;
reservation and artifact-producing jobs need `contents: write` for release assets.
Publication mutations additionally require the validated annotated-tag push context.
The separate cleanup command can run manually, after verifying completed publication.

Accepted candidate archives, acceptance evidence and documentation are copied into
this draft, with a checksum inventory recorded before upload and verified afterward.
Promotion retains deterministic multi-architecture archives, frozen notes/review and
append-only publication journals there too. **Keep these drafts while publication is
incomplete, including failed runs.** After the product release becomes public, the
publisher verifies its manifest, checksum, metadata, image digests and signature
checkpoints against the completed journal, then deletes only the internal draft.
The public manifest becomes the permanent completion record for deployments and
future compatibility baselines. Image digests and registry signatures remain intact.
Repository writers can see incomplete drafts; public visitors see product releases.

A cleanup failure leaves the public release intact and fails the publication job.
Use **Re-run all jobs** with this workflow to verify the completed manifest and retry cleanup;
completed releases skip candidate builds, validation and publication jobs. Their
deployment job can still run against the published digests. The standalone `recover`
command refuses to rebuild a completed release. A corrupted or missing completion
record stops the retry instead of accepting replacement binaries.

Rerun the original workflow after a failure. Accepted bytes are downloaded and hashed;
no accepted image is rebuilt. Once durable retention finishes, recovery no longer
requires Actions artifacts. A partial retention upload can use only the original
run's artifacts, with the already-recorded hashes. Missing, expired or changed bytes
stop publication. A different workflow run cannot build replacements for an existing
reservation. If reservation creation itself was interrupted, or accepted bytes were
lost before durable upload, inspect and recover the original inputs manually; do not
clear the reservation or replace a product tag to force a rebuild.

Existing public journals remain readable in place. Completed historical releases are
not edited, stripped of assets or republished by retries. The global publication lock,
exact image digest checks and non-regressing aliases remain in force. New public
uploads never use `--clobber`; a conflicting attachment fails rather than overwrites.

To clean up a completed release left by an earlier publisher, use its exact resolved
metadata (the `metadata` object in its public `release-manifest.json`):

```bash
python3 src/tools/release/release.py cleanup --repository Citadel-P/Citadel \
  --metadata /path/to/resolved-version.json --directory /tmp/release-cleanup
# After reviewing the dry-run result, repeat with --apply.
```

Cleanup is idempotent and refuses unpublished releases, incomplete journals and
mismatched provenance. It never deletes product releases, Git tags or registry
images. Older tagged workflows retain their original code; after cleanup, use the
current publisher for any further publication checks instead of rerunning their
old recovery logic. Legacy releases with public journals remain unchanged.

Publication recovery is separate from installation recovery. Users must back up
their control-plane state, keys and application data before an upgrade; GitHub
release files contain no installation data. Follow the
[upgrade and rollback guide](content/docs/operations/upgrade-and-rollback.mdx).
