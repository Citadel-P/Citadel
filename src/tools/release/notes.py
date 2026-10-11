#!/usr/bin/env python3
"""Pinned git-cliff notes for an explicit, validated release revision."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import urllib.request

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PIN = json.loads((HERE / "git-cliff-pin.json").read_text())
STABLE = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z")
TAG = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-dev\.([1-9][0-9]*))?\Z")


def run(*args):
    return subprocess.check_output([str(a) for a in args], text=True).strip()


def install():
    """Download the reviewed binary; verify the archive even on a cache hit."""
    cache = Path.home() / ".cache/citadel-tools" / ("git-cliff-" + PIN["version"])
    cache.mkdir(parents=True, exist_ok=True)
    archive = cache / "tool.tar.gz"
    if not archive.exists():
        with urllib.request.urlopen(PIN["url"], timeout=60) as response:
            data = response.read()
        if hashlib.sha256(data).hexdigest() != PIN["sha256"]:
            raise ValueError("git-cliff download checksum differs")
        archive.write_bytes(data)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != PIN["sha256"]:
        raise ValueError("Cached git-cliff checksum differs")
    binary = cache / "git-cliff"
    with tarfile.open(archive) as source:
        member = source.getmember(f"git-cliff-{PIN['version']}/git-cliff")
        if not member.isfile():
            raise ValueError("Invalid git-cliff binary")
        binary.write_bytes(source.extractfile(member).read())
    binary.chmod(0o755)
    if run(binary, "--version") != "git-cliff " + PIN["version"]:
        raise ValueError("git-cliff version differs from pin")
    return binary


def order(tag):
    match = TAG.fullmatch(tag)
    if not match:
        raise ValueError("Invalid release tag")
    return (*map(int, match.group(1, 2, 3)), 0 if match[4] else 1, int(match[4] or 0))


def predecessor(root, tag, revision):
    """Stable notes include every change since the preceding reachable stable."""
    tags = run("git", "-C", root, "tag", "--merged", revision).splitlines()
    eligible = [t for t in tags if TAG.fullmatch(t) and order(t) < order(tag)
                and (not STABLE.fullmatch(tag) or STABLE.fullmatch(t))]
    return max(eligible, key=order) if eligible else None


def generate(root, metadata, directory, binary, docs_url):
    tag, revision = "v" + metadata["displayVersion"], metadata["sourceRevision"]
    if not TAG.fullmatch(tag) or not re.fullmatch(r"[a-f0-9]{40}", revision):
        raise ValueError("Invalid release identity for notes")
    if run("git", "-C", root, "rev-parse", f"refs/tags/{tag}^{{commit}}") != revision:
        raise ValueError("Notes must target the exact tagged revision")
    if run(binary, "--version") != "git-cliff " + PIN["version"]:
        raise ValueError("git-cliff version differs from pin")
    if not docs_url.startswith("https://") or any(c.isspace() for c in docs_url):
        raise ValueError("Release notes require an HTTPS documentation URL")
    previous = predecessor(root, tag, revision)
    span = f"{previous}..{revision}" if previous else revision
    context = json.loads(run(binary, "--config", HERE / "cliff.toml", "--workdir", root,
                             "--no-exec", "--context", "--", span))
    review, seen = [], set()
    for entry in context:
        commits = []
        for commit in entry["commits"]:
            commit.setdefault("conventional", False)
            commit.setdefault("breaking", False)
            commit.setdefault("breaking_description", None)
            # git-cliff leaves nonconventional commits unparsed. Their explicit
            # breaking footers still belong in the public release notes.
            breaking = re.search(r"(?ms)^BREAKING[ -]CHANGE:\s*(.+)", commit.get("raw_message") or commit["message"])
            if breaking and not commit["breaking"]:
                commit["breaking"] = True
                commit["breaking_description"] = breaking[1].strip()
            # Merge titles can repeat a constituent commit. Keep distinct changes
            # and all breaking descriptions; no GitHub API/PR enrichment needed.
            key = (commit["message"], commit.get("breaking_description"), commit["breaking"])
            if key in seen:
                continue
            seen.add(key)
            subject = commit["message"].splitlines()[0].strip()
            unclear = re.fullmatch(r"(?i)(update[sd]?|changes?|fix(?:es)?|fix bugs?|misc(?:ellaneous)?|stuff|wip|cleanup)[.! ]*", subject)
            if unclear or not commit["conventional"] or "review" in (commit.get("group") or "").lower():
                review.append(dict(commit=commit["id"], message=commit["message"],
                                   reason="Unclear, nonconventional or unclassified commit; review before publication"))
            if commit["breaking"]:
                commit["group"] = "<!-- 0 -->Breaking changes"
            elif "review" in (commit.get("group") or "").lower():
                commit["group"] = "<!-- 5 -->Other changes"
            commit["extra"] = dict(breaking=commit["breaking"], breakingDescription=commit["breaking_description"])
            commits.append(commit)
        entry["commits"] = commits
    directory.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as temporary:
        path = Path(temporary) / "context.json"
        path.write_text(json.dumps(context))
        body = run(binary, "--config", HERE / "cliff.toml", "--no-exec", "--from-context", path)
    compare = (f"https://github.com/Citadel-P/Citadel/compare/{previous}...{tag}" if previous
               else f"https://github.com/Citadel-P/Citadel/commits/{tag}")
    docs = docs_url.rstrip("/")
    body = body or "No user-facing changes in this release."
    text = f"{body}\n\n[Full changelog]({compare})\n\n## Install & upgrade\n\n"
    text += f"[Installation]({docs}/docs/getting-started/install) · [Upgrade and rollback]({docs}/docs/operations/upgrade-and-rollback)\n\n"
    text += "Update Agents before Core. Images support Linux amd64 and arm64. Immutable digests and signature verification details are in `release-manifest.json`.\n\n"
    text += "| Image | GHCR" + (" | Docker Hub" if metadata["releaseEligible"] else "") + " |\n"
    text += "| --- | ---" + (" | ---" if metadata["releaseEligible"] else "") + " |\n"
    for component, ghcr, hub in [("Core", "citadel", "citadel"), ("Agent", "citadel.agent", "citadel-agent")]:
        text += f"| {component} | `ghcr.io/citadel-p/{ghcr}:{metadata['displayVersion']}`"
        if metadata["releaseEligible"]:
            text += f" | `docker.io/citadelplane/{hub}:{metadata['displayVersion']}`"
        text += " |\n"
    (directory / "release-notes.md").write_text(text)
    report = dict(metadata=metadata, previousTag=previous, revisionRange=span,
                  gitCliffVersion=PIN["version"], review=review)
    (directory / "release-review.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["install", "generate"])
    parser.add_argument("--metadata", type=Path)
    parser.add_argument("--directory", type=Path)
    args = parser.parse_args()
    binary = install()
    if args.command == "install":
        print(binary)
        return
    if not args.metadata or not args.directory:
        parser.error("generate requires --metadata and --directory")
    report = generate(ROOT, json.loads(args.metadata.read_text()), args.directory, binary, os.environ["DOCS_SITE_URL"])
    if report["review"]:
        print(f"::warning::{len(report['review'])} changelog commits need review; see release-review.json in release-inputs")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as output:
            output.write("## Release notes preview\n\n" + (args.directory / "release-notes.md").read_text())
            output.write(f"\n\nChangelog review: **{len(report['review'])} commits**. See `release-review.json` in release-inputs.\n")
            for item in report["review"]:
                output.write(f"- [{item['commit'][:7]}](https://github.com/Citadel-P/Citadel/commit/{item['commit']}): {item['message'].splitlines()[0]}\n")


if __name__ == "__main__":
    main()
