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
Mutation commands additionally require the validated annotated-tag push context.

Accepted candidate archives, acceptance evidence and documentation are copied into
this draft, with a checksum inventory recorded before upload and verified afterward.
Promotion retains deterministic multi-architecture archives, frozen notes/review and
append-only publication journals there too. **Retain recovery drafts indefinitely;
exclude them from release cleanup automation.** There is no automatic deletion or
migration of historical releases. Repository writers can see the drafts; public
visitors see only completed product releases. Keep an independent backup if protection
from administrator deletion or loss of the GitHub repository is required.

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
