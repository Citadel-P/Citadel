#!/usr/bin/env python3
"""Coordinated, journaled promotion. No external writes unless --apply is passed.

Requires gh, skopeo and cosign. Candidate artifacts are retained in a draft GitHub
release before registry writes. Each journal revision is an append-only asset.
"""
import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("citadel_version", ROOT / "src/tools/build/version.py")
version = importlib.util.module_from_spec(spec)
spec.loader.exec_module(version)
POLICY = json.loads(Path(__file__).with_name("policy.json").read_text())
IMAGE = re.compile(r"[a-z0-9][a-z0-9./_-]*@sha256:[a-f0-9]{64}\Z")


def canonical(data):
    return json.dumps(data, sort_keys=True, separators=(",", ":")).encode()


def digest(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def semver(value):
    version.product_version(value)
    return tuple(int(part) for part in value.split("."))


def development(metadata):
    return metadata.get("developmentReleaseEligible") is True


def repositories(metadata):
    # Development publication uses GHCR only; stable destinations are unchanged.
    return ({c: [repos[0]] for c, repos in POLICY["registries"].items()}
            if development(metadata) else POLICY["registries"])


def development_order(value):
    match = version.DEVELOPMENT.fullmatch(value)
    if not match:
        raise ValueError("Expected MAJOR.MINOR.PATCH-dev.N with a positive N")
    return (*semver(match[1]), int(match[2]))


def documentation_order(value):
    """One public docs site follows SemVer across development and stable releases."""
    if version.DEVELOPMENT.fullmatch(value):
        *product, number = development_order(value)
        return (*product, 0, number)
    return (*semver(value), 1, 0)


def documentation_eligible(metadata, backend):
    current = documentation_order(metadata["displayVersion"])
    # Both channels publish to GHCR. Never require Docker Hub credentials for dev.
    published = (tag for repos in POLICY["registries"].values()
                 for tag in backend.tags(repos[0])
                 if version.STABLE.fullmatch(tag) or version.DEVELOPMENT.fullmatch(tag))
    return not any(documentation_order(tag) > current for tag in published)


def development_compatibility():
    # No stable upgrade guarantee is claimed for the development channel.
    return dict(bootstrap=False, agent=None, core=None, protocolVersion=POLICY["protocolVersion"],
                architectures=POLICY["architectures"], agentFirstUpgrades=POLICY["agentFirstUpgrades"])


def aliases(product, component, releases):
    current = semver(product)
    versions = [semver(item) for item in releases]
    result = []
    if not any(v[:2] == current[:2] and v > current for v in versions):
        result.append(f"{current[0]}.{current[1]}")
    if not any(v[0] == current[0] and v > current for v in versions):
        result.append(str(current[0]))
    if component == "core" and not any(v > current for v in versions):
        result.append("latest")
    return result


class Backend:
    def __init__(self, repository, apply=False):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
            raise ValueError("Invalid GitHub repository")
        self.repository, self.apply = repository, apply

    def command(self, *args, write=False):
        if write and not self.apply:
            raise RuntimeError("External mutation blocked: dry run")
        return version.run(*args)

    def releases(self):
        pages = json.loads(self.command("gh", "api", "--paginate", "--slurp", f"repos/{self.repository}/releases?per_page=100"))
        return [release for page in pages for release in page]

    def release(self, tag):
        return next((item for item in self.releases() if item["tag_name"] == tag), None)

    def asset(self, release, name, destination):
        item = next((a for a in release["assets"] if a["name"] == name), None)
        if item is None:
            raise RuntimeError(f"Missing durable release artifact: {name}")
        with Path(destination).open("wb") as stream:
            subprocess.run(["gh", "api", "-H", "Accept: application/octet-stream",
                            f"repos/{self.repository}/releases/assets/{item['id']}"], stdout=stream, check=True)

    def load(self, tag):
        release = self.release(tag)
        if release is None:
            return None
        records = sorted(a["name"] for a in release["assets"] if re.fullmatch(r"release-record-[0-9]{6}\.json", a["name"]))
        if not records:
            raise RuntimeError("Release exists without a recovery journal; inspect manually before proceeding")
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / records[-1]
            self.asset(release, records[-1], path)
            return json.loads(path.read_text())

    def save(self, record):
        if not self.apply:
            raise RuntimeError("Journal mutation blocked: dry run")
        tag = record["tag"]
        record["sequence"] = record.get("sequence", 0) + 1
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / f"release-record-{record['sequence']:06}.json"
            path.write_text(json.dumps(record, indent=2) + "\n")
            if self.release(tag) is None:
                # Tag already exists and was independently validated; gh must not create one.
                self.command("gh", "release", "create", tag, str(path), "--repo", self.repository,
                             "--verify-tag", "--draft", "--title", f"Citadel {tag} (publication in progress)",
                             "--notes", "Release journal and candidate artifacts are retained here for recovery.", write=True)
            else:
                self.command("gh", "release", "upload", tag, str(path), "--repo", self.repository, write=True)

    def upload(self, tag, path):
        self.command("gh", "release", "upload", tag, path, "--repo", self.repository, write=True)

    def action_artifact(self, record, name, destination):
        # Only needed if draft asset retention was interrupted before registry
        # publication. The original run is immutable input; never use a retry's
        # newly built output. Expired/missing artifacts stop recovery.
        artifact = ("candidates-" + name.split("-")[1].split(".")[0]
                    if name.startswith(("core-", "agent-", "evidence-")) else "release-documentation")
        with tempfile.TemporaryDirectory() as temporary:
            self.command("gh", "run", "download", str(record["sourceRunId"]), "--repo", self.repository,
                         "--name", artifact, "--dir", temporary)
            source = Path(temporary) / name
            if not source.is_file():
                raise ValueError(f"Original artifact unavailable: {name}; do not rebuild this release")
            shutil.copyfile(source, destination)

    def recover_validation_artifact(self, name, directory, metadata):
        """Reuse accepted pre-publication outputs when rerunning the same run."""
        run_id = os.environ["GITHUB_RUN_ID"]
        pages = json.loads(self.command("gh", "api", "--paginate", "--slurp",
                                       f"repos/{self.repository}/actions/runs/{run_id}/artifacts?per_page=100"))
        matches = [a for page in pages for a in page["artifacts"] if a["name"] == name]
        if not matches:
            # If validation completed previously, losing its artifact must not
            # silently cause a replacement build. Fail before any publication.
            for attempt in range(1, int(os.environ.get("GITHUB_RUN_ATTEMPT", "1"))):
                jobs = json.loads(self.command("gh", "api", "--paginate", "--slurp",
                    f"repos/{self.repository}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100"))
                arch = name.removeprefix("candidates-")
                for page in jobs:
                    for job in page["jobs"]:
                        relevant = (job["name"] == "Validate and retain release documentation" if name == "release-documentation"
                                    else job["name"].endswith("Tested Core and Agent linux/" + arch))
                        accepted_step = ("Build and validate static sites from this commit" if name == "release-documentation"
                                         else "Exercise exact candidates and supported upgrade directions")
                        if relevant and any(s["name"] == accepted_step and s["conclusion"] == "success" for s in job["steps"]):
                            raise ValueError("Previously accepted artifact is unavailable; do not rebuild this release")
            return False
        if any(a["expired"] for a in matches):
            raise ValueError("Original validation artifact expired; do not rebuild this release")
        with tempfile.TemporaryDirectory() as temporary:
            self.command("gh", "run", "download", run_id, "--repo", self.repository,
                         "--name", name, "--dir", temporary)
            source = Path(temporary)
            if name == "release-documentation":
                manifest = json.loads((source / "documentation-inputs.json").read_text())
                hashes = manifest["artifacts"]
            else:
                arch = name.removeprefix("candidates-")
                manifest = json.loads((source / f"evidence-{arch}.json").read_text())
                hashes = {f"{c}-{arch}.oci.tar": h for c, h in manifest["artifacts"].items()}
            if manifest["metadata"] != metadata:
                raise ValueError("Previous validation artifact metadata differs")
            for filename, expected in hashes.items():
                if Path(filename).name != filename or version.sha256(source / filename) != expected:
                    raise ValueError("Previous validation artifact identity differs")
            for path in source.iterdir():
                if path.is_file():
                    shutil.copyfile(path, directory / path.name)
        return True

    def tags(self, repository):
        try:
            return json.loads(self.command("skopeo", "list-tags", "docker://" + repository))["Tags"] or []
        except RuntimeError as error:
            raise RuntimeError(f"Cannot list image tags for {repository}: {error}") from error

    def manifest(self, reference):
        # Raw bytes are hashed, not reformatted JSON. Missing/auth/network errors
        # remain fatal; existence is established separately through list-tags.
        result = subprocess.run(["skopeo", "inspect", "--raw", "docker://" + reference], capture_output=True, check=True)
        return result.stdout

    def existing(self, repository, tag):
        return digest(self.manifest(repository + ":" + tag)) if tag in self.tags(repository) else None

    def copy(self, source, destination):
        with oci_source(source) as resolved:
            self.command("skopeo", "copy", "--all", "--preserve-digests", resolved, "docker://" + destination, write=True)

    def sign(self, reference, identity):
        self.command("cosign", "sign", "--yes", reference, write=True)
        self.command("cosign", "verify", "--certificate-identity", identity,
                     "--certificate-oidc-issuer", "https://token.actions.githubusercontent.com", reference)


def check_metadata(metadata):
    if metadata.get("schemaVersion") != 1 or (metadata.get("releaseEligible") is not True and not development(metadata)):
        raise ValueError("Coordinated publication requires release-eligible resolver output")
    product = version.product_version(metadata["productVersion"])
    display = metadata.get("displayVersion", "")
    if development(metadata):
        match = version.DEVELOPMENT.fullmatch(display)
        if not match or match[1] != product or metadata.get("releaseEligible") is not False:
            raise ValueError("Development metadata must match its product version and cannot be stable-eligible")
    elif display != product:
        raise ValueError("Stable metadata must match product version")
    if metadata.get("dirty") is not False:
        raise ValueError("Publication metadata must be clean")
    if not version.SHA.fullmatch(metadata.get("sourceRevision", "")):
        raise ValueError("Missing plain full source SHA")
    height = metadata.get("versionHeight")
    if type(height) is not int or height < 0 or metadata.get("nbgvPublicRelease") is not True:
        raise ValueError("Invalid NBGV release metadata")
    if metadata.get("toolIdentity") != version.tool_identity() or metadata.get("identityStatus") != "resolved":
        raise ValueError("Unexpected version tool or fallback identity")
    if metadata["informationalVersion"] != f"{display}+height.{height}.sha.{metadata['sourceRevision']}":
        raise ValueError("Unexpected informational mapping")


def preflight(metadata, backend, environ):
    check_metadata(metadata)
    if environ.get("CITADEL_RUST_AGENT_RELEASE_ENABLED") != "true":
        raise ValueError("Coordinated release blocked: CITADEL_RUST_AGENT_RELEASE_ENABLED must be true")
    if backend.repository.lower() != "citadel-p/citadel":
        raise ValueError("Publication destinations require the Citadel-P/Citadel repository")
    if not environ.get("DOCS_SITE_URL"):
        raise ValueError("Release blocked: DOCS_SITE_URL is not configured")
    for key in ["DOCS_SITE_URL", "API_DOCS_URL"]:
        if environ.get(key) and not re.fullmatch(r"https://[^\s]+", environ[key]):
            raise ValueError(f"{key} must be a canonical HTTPS URL")
    if development(metadata):
        recorded = backend.load("v" + metadata["displayVersion"])
        if recorded and recorded["metadata"] != metadata:
            raise ValueError("Recorded release identity differs from current inputs")
        return recorded["compatibility"] if recorded else development_compatibility()
    if environ.get("DOCKERHUB_NAMESPACE") != "citadelplane":
        raise ValueError("DOCKERHUB_NAMESPACE must match the reviewed destinations")
    for key in ["DOCKERHUB_USERNAME", "DOCKERHUB_TOKEN"]:
        if not environ.get(key):
            raise ValueError(f"Release blocked: {key} is not configured")
    recorded = backend.load("v" + metadata["productVersion"])
    if recorded:
        if recorded["metadata"] != metadata:
            raise ValueError("Recorded release identity differs from current inputs")
        # Baselines/bootstrap are frozen once candidates are accepted. Changing
        # repository variables for a later release must not change an old retry.
        return recorded["compatibility"]
    agent, core = environ.get("CITADEL_AGENT_ROLLBACK_IMAGE", ""), environ.get("CITADEL_CORE_ROLLBACK_IMAGE", "")
    bootstrap = environ.get("CITADEL_FIRST_RUST_RELEASE", "") == metadata["productVersion"]
    current = "v" + metadata["productVersion"]
    if not agent or not core:
        if not bootstrap or agent or core:
            raise ValueError("Both digest-pinned Core and Agent baselines are required, or explicit first-release bootstrap")
        # Prior exact tags in either registry prohibit waiving the baseline.
        for repositories in POLICY["registries"].values():
            for repository in repositories:
                if any(version.STABLE.fullmatch(tag) and tag != metadata["productVersion"] for tag in backend.tags(repository)):
                    raise ValueError("Bootstrap forbidden: released images already exist")
        if any(r["tag_name"] != current and version.STABLE.fullmatch(r["tag_name"].removeprefix("v")) for r in backend.releases()):
            raise ValueError("Bootstrap forbidden: a previous release exists")
    else:
        bootstrap = False
        for image in [agent, core]:
            if not IMAGE.fullmatch(image):
                raise ValueError("Baseline must be an explicit repository@sha256 digest")
            manifest = json.loads(backend.manifest(image))
            arches = {m.get("platform", {}).get("architecture") for m in manifest.get("manifests", [])
                      if m.get("platform", {}).get("os") == "linux"}
            if not set(POLICY["architectures"]).issubset(arches):
                raise ValueError("Baseline lacks a required Linux architecture")
    if not bootstrap:
        for component, image in [("agent", agent), ("core", core)]:
            # Only an earlier completed coordinated Rust release can establish
            # the supported baseline. An arbitrary digest is not release evidence.
            found = False
            for prior in backend.releases():
                name = prior["tag_name"]
                if prior["draft"] or not name.startswith("v") or not version.STABLE.fullmatch(name[1:]):
                    continue
                if semver(name[1:]) >= semver(metadata["productVersion"]):
                    continue
                journal = backend.load(name)
                if (journal and journal.get("status") == "complete"
                        and image in [repo + "@" + journal["indexes"][component]
                                      for repo in POLICY["registries"][component]]):
                    found = True
                    break
            if not found:
                raise ValueError(f"{component} baseline is not an earlier completed Rust release")
    return dict(bootstrap=bootstrap, agent=agent or None, core=core or None,
                protocolVersion=POLICY["protocolVersion"], architectures=POLICY["architectures"],
                agentFirstUpgrades=POLICY["agentFirstUpgrades"])


def unpack(archive, destination):
    with tarfile.open(archive) as source:
        for member in source.getmembers():
            target = destination / member.name
            if not target.resolve().is_relative_to(destination.resolve()) or not (member.isfile() or member.isdir()):
                raise ValueError("Unsafe candidate archive")
        source.extractall(destination)


@contextmanager
def oci_source(source):
    # Skopeo's oci-archive reader may chown files while extracting root-owned
    # BuildKit archives. Extract safely without restoring ownership, then copy
    # the identical OCI blobs via directory transport on an unprivileged runner.
    if not source.startswith("oci-archive:"):
        yield source
        return
    with tempfile.TemporaryDirectory() as temporary:
        layout = Path(temporary)
        unpack(Path(source.removeprefix("oci-archive:")), layout)
        yield "oci:" + str(layout)


def platform_manifest(layout, arch):
    entries = json.loads((layout / "index.json").read_text())["manifests"]
    if len(entries) != 1:
        raise ValueError("Expected one platform manifest; disable provenance/sbom attestations for this transport")
    entry = entries[0]
    while entry["mediaType"] in ["application/vnd.oci.image.index.v1+json",
                                 "application/vnd.docker.distribution.manifest.list.v2+json"]:
        raw = (layout / "blobs/sha256" / entry["digest"].split(":")[1]).read_bytes()
        if digest(raw) != entry["digest"]:
            raise ValueError("Candidate index checksum mismatch")
        nested = json.loads(raw)
        if len(nested["manifests"]) != 1:
            raise ValueError("Unexpected multi-platform/attestation candidate input")
        entry = nested["manifests"][0]
    raw = (layout / "blobs/sha256" / entry["digest"].split(":")[1]).read_bytes()
    if digest(raw) != entry["digest"]:
        raise ValueError("Candidate manifest checksum mismatch")
    manifest = json.loads(raw)
    config_raw = (layout / "blobs/sha256" / manifest["config"]["digest"].split(":")[1]).read_bytes()
    if digest(config_raw) != manifest["config"]["digest"]:
        raise ValueError("Candidate config checksum mismatch")
    config = json.loads(config_raw)
    if config["os"] != "linux" or config["architecture"] != arch:
        raise ValueError("Candidate architecture differs from its assigned runner")
    entry["platform"] = {"os": "linux", "architecture": arch}
    entry.pop("annotations", None)
    return entry, manifest["config"]["digest"]


def assemble(component, directory):
    """One OCI index, whose identical bytes are copied to every registry."""
    target = directory / f"{component}.oci.tar"
    with tempfile.TemporaryDirectory() as temporary:
        layout = Path(temporary) / "layout"
        blobs = layout / "blobs/sha256"
        blobs.mkdir(parents=True)
        manifests = []
        for arch in POLICY["architectures"]:
            child = Path(temporary) / arch
            child.mkdir()
            unpack(directory / f"{component}-{arch}.oci.tar", child)
            entry, _ = platform_manifest(child, arch)
            for blob in (child / "blobs/sha256").iterdir():
                if version.sha256(blob) != blob.name:
                    raise ValueError("Candidate blob checksum mismatch")
                shutil.copyfile(blob, blobs / blob.name)
            manifests.append(entry)
        raw = canonical({"schemaVersion": 2, "mediaType": "application/vnd.oci.image.index.v1+json", "manifests": manifests})
        index_digest = digest(raw)
        (blobs / index_digest.split(":")[1]).write_bytes(raw)
        (layout / "oci-layout").write_text('{"imageLayoutVersion":"1.0.0"}')
        (layout / "index.json").write_bytes(canonical({"schemaVersion": 2, "manifests": [{
            "mediaType": "application/vnd.oci.image.index.v1+json", "digest": index_digest,
            "size": len(raw), "annotations": {"org.opencontainers.image.ref.name": "release"}}]}))
        with tarfile.open(target, "w") as output:
            for path in sorted(layout.rglob("*")):
                if path.is_file():
                    info = output.gettarinfo(str(path), arcname=str(path.relative_to(layout)))
                    info.mtime = 0
                    info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    with path.open("rb") as source:
                        output.addfile(info, source)
    return index_digest


def checkpoint(backend, record, key, value=True):
    record["steps"][key] = value
    backend.save(record)


def promote(backend, record, directory):
    """Resume idempotently; verify remote exact tags even for journaled steps."""
    if not backend.apply:
        return {"dryRun": True, "tag": record["tag"], "sequence": ["agent exact", "core exact", "aliases", "docs"]}
    if not record["steps"].get("artifacts-retained"):
        raise ValueError("Durable artifacts must be retained before registry mutation")
    product = record["metadata"]["displayVersion"]
    destinations = repositories(record["metadata"])
    for component in ["agent", "core"]:
        expected = record["indexes"][component]
        for repository in destinations[component]:
            exact = repository + ":" + product
            existing = backend.existing(repository, product)
            if existing is not None and existing != expected:
                raise ValueError(f"Immutable exact tag conflict: {exact}")
            if existing is None:
                backend.copy(f"oci-archive:{directory / (component + '.oci.tar')}", exact)
            if backend.existing(repository, product) != expected:
                raise ValueError(f"Cross-registry digest mismatch: {exact}")
            checkpoint(backend, record, "exact:" + exact, expected)
            backend.sign(repository + "@" + expected, record["signingIdentity"])
            checkpoint(backend, record, "signed:" + exact, expected)
    # All destinations for both components are verified before any alias update.
    record["status"] = "exact-verified"
    backend.save(record)
    for component in ["agent", "core"]:
        expected = record["indexes"][component]
        for repository in destinations[component]:
            # Include tags in both registries: an interrupted newer release must
            # not be rolled back by retrying an older release in the other registry.
            pattern = version.DEVELOPMENT if development(record["metadata"]) else version.STABLE
            published = [tag for repos in destinations.values() for repo in repos
                         for tag in backend.tags(repo) if pattern.fullmatch(tag)]
            if development(record["metadata"]):
                newer = any(development_order(t) > development_order(product) for t in published)
                eligible_aliases = [] if newer else ["dev"]
            else:
                eligible_aliases = aliases(product, component, published)
            for alias in eligible_aliases:
                reference = repository + ":" + alias
                backend.copy("docker://" + repository + "@" + expected, reference)
                if backend.existing(repository, alias) != expected:
                    raise ValueError(f"Alias digest mismatch: {reference}")
                checkpoint(backend, record, "alias:" + reference, expected)
    record["status"] = "images-verified"
    backend.save(record)
    return record


def retain(backend, record, directory):
    """Fill missing draft assets after an interrupted upload, before promotion."""
    available = {a["name"] for a in backend.release(record["tag"])["assets"]}
    for name, expected in record["artifacts"].items():
        path = directory / name
        if version.sha256(path) != expected:
            raise ValueError(f"Artifact checksum mismatch before retention: {name}")
        if name not in available:
            backend.upload(record["tag"], path)
    # Independently download and hash retained bytes before any registry write.
    with tempfile.TemporaryDirectory() as temporary:
        recover(backend, record["tag"], Path(temporary), durable_only=True)
    checkpoint(backend, record, "artifacts-retained")


def recover(backend, tag, directory, durable_only=False):
    record = backend.load(tag)
    if record is None:
        return None
    release = backend.release(tag)
    directory.mkdir(parents=True, exist_ok=True)
    available = {a["name"] for a in release["assets"]}
    deferred = []
    for name, expected in record["artifacts"].items():
        if Path(name).name != name:
            raise ValueError("Invalid journal artifact name")
        path = directory / name
        if name in available:
            backend.asset(release, name, path)
        elif durable_only or record["steps"].get("artifacts-retained"):
            raise ValueError(f"Durable artifact missing: {name}; do not rebuild this release")
        elif name in ["agent.oci.tar", "core.oci.tar"]:
            deferred.append(name)
            continue
        else:
            backend.action_artifact(record, name, path)
        if version.sha256(path) != expected:
            raise ValueError(f"Durable artifact identity lost: {name}; do not rebuild this release")
    # Reconstruct only deterministic index packaging, never image binaries.
    for name in deferred:
        component = name.split(".")[0]
        if assemble(component, directory) != record["indexes"][component]:
            raise ValueError("Recovered index identity mismatch")
        if version.sha256(directory / name) != record["artifacts"][name]:
            raise ValueError("Recovered archive identity mismatch")
    return record


def validate_evidence(directory, metadata, compatibility):
    for arch in POLICY["architectures"]:
        evidence = json.loads((directory / f"evidence-{arch}.json").read_text())
        if (evidence["metadata"] != metadata or evidence["compatibility"] != compatibility
                or evidence["passed"] is not True or evidence["architecture"] != arch):
            raise ValueError("Candidate acceptance evidence does not match release")
        required = {"independent-binary-and-oci-versions", "core-runtime", "agent-image",
                    "agent-compatibility", "candidate-acceptance"}
        if not required.issubset(evidence["checks"]):
            raise ValueError("Missing required candidate checks")
        if set(evidence["mixedVersionChecks"]) != {c for c in ["agent", "core"] if compatibility.get(c)}:
            raise ValueError("Missing supported mixed-version checks")
        for component in ["core", "agent"]:
            if evidence["artifacts"][component] != version.sha256(directory / f"{component}-{arch}.oci.tar"):
                raise ValueError("Candidate was changed after acceptance")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["preflight", "recover", "promote", "docs", "finalize"])
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--artifact", choices=["candidates-amd64", "candidates-arm64", "release-documentation"])
    args = parser.parse_args()
    metadata = json.loads(args.metadata.read_text())
    backend = Backend(args.repository, args.apply)
    check_metadata(metadata)
    tag = "v" + metadata["displayVersion"]
    if args.apply:
        if (os.environ.get("GITHUB_EVENT_NAME") != "push" or os.environ.get("GITHUB_REF") != "refs/tags/" + tag
                or os.environ.get("GITHUB_SHA") != metadata["sourceRevision"]):
            raise ValueError("Writes require the validated release-tag push context; manual runs are read-only")
        if version.git(ROOT, "cat-file", "-t", "refs/tags/" + tag) != "tag":
            raise ValueError("Publication requires an annotated tag")
        if version.git(ROOT, "rev-parse", f"refs/tags/{tag}^{{commit}}") != metadata["sourceRevision"]:
            raise ValueError("Tag identity changed")
    if args.command == "recover":
        data = recover(backend, tag, args.directory)
        if data and data["metadata"] != metadata:
            raise ValueError("Recorded release identity differs from current inputs")
        restored = data is not None
        if not restored and args.artifact:
            restored = backend.recover_validation_artifact(args.artifact, args.directory, metadata)
        print(json.dumps({"recovered": restored}))
        return
    compatibility = preflight(metadata, backend, os.environ)
    if args.command == "preflight":
        print(json.dumps(compatibility, indent=2))
        return
    record = backend.load(tag)
    if args.command == "docs":
        if not record or record["status"] not in ["images-verified", "complete"]:
            raise ValueError("Documentation requires verified exact images")
        eligible = documentation_eligible(metadata, backend)
        print("eligible=" + str(eligible).lower())
        return
    if args.command == "finalize":
        if not args.apply:
            print(json.dumps({"dryRun": True, "tag": tag}))
            return
        if not record or record["status"] not in ["images-verified", "complete"]:
            raise ValueError("Exact artifacts and aliases must be verified before finalizing")
        result = os.environ.get("CITADEL_DOCS_RESULT", "")
        eligible = documentation_eligible(metadata, backend)
        if eligible and result.rstrip("/") != os.environ["DOCS_SITE_URL"].rstrip("/"):
            raise ValueError("Latest release requires a successful documentation deployment to DOCS_SITE_URL")
        if not eligible and result != "skipped-older-release":
            raise ValueError("Older release must not replace default documentation")
        record["status"] = "complete"
        record["steps"]["documentation"] = result
        backend.save(record)
        if development(metadata):
            backend.command("gh", "release", "edit", tag, "--repo", args.repository, "--draft=false",
                            "--prerelease", "--latest=false", "--title", f"Citadel {tag}", write=True)
            return
        published = [r["tag_name"][1:] for r in backend.releases() if not r["draft"] and version.STABLE.fullmatch(r["tag_name"][1:])]
        newest = not any(semver(v) > semver(metadata["productVersion"]) for v in published)
        backend.command("gh", "release", "edit", tag, "--repo", args.repository, "--draft=false",
                        "--title", f"Citadel {tag}", "--latest=" + str(newest).lower(), write=True)
        return
    if record:
        if record["metadata"] != metadata or record["compatibility"] != compatibility:
            raise ValueError("Release inputs differ from durable record")
        recover(backend, tag, args.directory)
    else:
        indexes = {component: assemble(component, args.directory) for component in ["agent", "core"]}
        required = [f"{c}-{a}.oci.tar" for c in ["agent", "core"] for a in POLICY["architectures"]]
        required += ["agent.oci.tar", "core.oci.tar", "docs.tar", "api-docs.tar"]
        required += [f"evidence-{a}.json" for a in POLICY["architectures"]]
        validate_evidence(args.directory, metadata, compatibility)
        record = dict(schemaVersion=1, sequence=0, tag=tag, metadata=metadata,
                      compatibility=compatibility, indexes=indexes, status="prepared", steps={},
                      sourceRunId=os.environ.get("GITHUB_RUN_ID"),
                      evidence={a: json.loads((args.directory / f"evidence-{a}.json").read_text()) for a in POLICY["architectures"]},
                      artifacts={name: version.sha256(args.directory / name) for name in required},
                      signingIdentity=f"https://github.com/{args.repository}/.github/workflows/ci.yml@refs/tags/{tag}")
        if args.apply:
            backend.save(record)  # durable journal BEFORE the first public registry mutation
    if args.apply and not record["steps"].get("artifacts-retained"):
        retain(backend, record, args.directory)
    print(json.dumps(promote(backend, record, args.directory), indent=2))


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, ValueError, OSError, subprocess.CalledProcessError, KeyError) as error:
        print(f"Release blocked: {error}", file=sys.stderr)
        sys.exit(1)
