#!/usr/bin/env python3
"""Pinned native NBGV installation, version resolution, and annotated tag preparation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
PIN = json.loads(Path(__file__).with_name("nbgv-pin.json").read_text())
STABLE = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z")
DEVELOPMENT = re.compile(r"((?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*))-dev\.([1-9][0-9]*)\Z")
SHA = re.compile(r"[a-f0-9]{40}\Z")
SHORT_LENGTH = 12


def run(*args, cwd=None, env=None):
    result = subprocess.run([str(a) for a in args], cwd=cwd, env=env, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        raise RuntimeError(f"{args[0]} {args[1] if len(args) > 1 else ''} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def git(root, *args):
    return run("git", "-C", root, *args)


def product_version(value):
    if not isinstance(value, str) or not STABLE.fullmatch(value):
        raise ValueError("version.json must contain a strict three-part stable SemVer without leading zeroes")
    return value


def tool_identity():
    return f"{PIN['repository']}@{PIN['revision']}"


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tool_dir():
    channel = re.search(r'^channel\s*=\s*"([^"]+)"', (ROOT / "rust-toolchain.toml").read_text(), re.M)[1]
    cache = Path(os.environ.get("CITADEL_TOOL_CACHE", Path.home() / ".cache" / "citadel-tools"))
    return cache / f"nbgv-{PIN['revision']}-{channel}-{platform.system()}-{platform.machine()}", channel


def nbgv(install=False):
    directory, channel = tool_dir()
    binary = directory / "bin" / ("nbgv.exe" if os.name == "nt" else "nbgv")
    receipt = directory / "receipt.json"
    if binary.exists() and receipt.exists():
        expected = {"pin": PIN, "toolchain": channel, "binarySha256": sha256(binary)}
        if json.loads(receipt.read_text()) == expected and run(binary, "--version") == PIN["cliVersion"]:
            return binary
        raise RuntimeError(f"NBGV cache verification failed; remove {directory} and reinstall")
    if not install:
        raise RuntimeError("Pinned NBGV is missing. Run python3 src/tools/build/version.py install")
    directory.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="citadel-nbgv-") as temporary:
        source = Path(temporary)
        run("git", "init", "--quiet", source)
        git(source, "fetch", "--quiet", "--depth=1", PIN["repository"], PIN["revision"])
        git(source, "checkout", "--quiet", "--detach", "FETCH_HEAD")
        if git(source, "rev-parse", "HEAD") != PIN["revision"]:
            raise RuntimeError("NBGV source pin mismatch")
        lock = source / "src/nerdbank-gitversioning-rs/Cargo.lock"
        if sha256(lock) != PIN["lockSha256"]:
            raise RuntimeError("NBGV dependency lock mismatch")
        subprocess.run(["cargo", f"+{channel}", "install", "--locked", "--path",
                        str(source / PIN["packagePath"]), "--root", str(directory)], check=True)
    if run(binary, "--version") != PIN["cliVersion"]:
        raise RuntimeError("Unexpected native NBGV CLI version")
    receipt.write_text(json.dumps({"pin": PIN, "toolchain": channel, "binarySha256": sha256(binary)}, indent=2) + "\n")
    return binary


def fallback(root):
    product = product_version(json.loads((root / "version.json").read_text())["version"])
    return dict(schemaVersion=1, productVersion=product, displayVersion=f"{product}-dev.unknown",
                informationalVersion=f"{product}-dev.unknown+source.unknown", sourceRevision=None,
                versionHeight=None, dirty=None, nbgvPublicRelease=None, releaseEligible=False,
                identityStatus="explicit-fallback", toolIdentity=None, developmentReleaseEligible=False,
                publicationEligible=False)


def resolve(root, event="local", ref="", expected_sha=None, allow_fallback=False):
    release = event == "push" and ref.startswith("refs/tags/")
    if allow_fallback:
        if release:
            raise ValueError("Release context cannot use fallback identity")
        return fallback(root)
    if git(root, "rev-parse", "--is-shallow-repository") != "false":
        raise ValueError("Shallow history: fetch --unshallow --tags, or explicitly use --fallback for development")
    source = git(root, "rev-parse", "HEAD")
    if not SHA.fullmatch(source) or (expected_sha and source != expected_sha):
        raise ValueError("Built source does not match the expected full commit SHA")
    product = product_version(json.loads(git(root, "show", "HEAD:version.json"))["version"])
    working_product = product_version(json.loads((root / "version.json").read_text())["version"])
    if working_product != product:
        raise ValueError("Commit version.json product-version changes before resolving identity, or explicitly select --fallback")
    dirty = bool(git(root, "status", "--porcelain", "--untracked-files=all"))
    # Ambient CI variables must not secretly change NBGV's interpretation. It
    # discovers public tags at HEAD itself; Citadel separately checks context.
    env = {key: value for key, value in os.environ.items() if not key.startswith(
        ("GITHUB_", "GITLAB_", "CI_", "BUILD_", "SYSTEM_", "APPVEYOR_", "TEAMCITY_", "TRAVIS_"))
           and key not in ("PublicRelease", "CI", "TF_BUILD")}
    raw = json.loads(run(nbgv(), "get-version", "--format", "json", "--project", root, env=env))
    height, public = raw.get("VersionHeight"), raw.get("PublicRelease")
    if type(height) is not int or height < 0 or type(public) is not bool:
        raise ValueError("Pinned NBGV JSON contract changed: VersionHeight/PublicRelease")
    if raw.get("SimpleVersion") != product:
        raise ValueError("NBGV SimpleVersion differs from the committed product version")
    development = False
    if release:
        tag = ref.removeprefix("refs/tags/")
        match = DEVELOPMENT.fullmatch(tag[1:]) if tag.startswith("v") else None
        development = match is not None
        base = match[1] if match else tag[1:]
        if not tag.startswith("v") or product_version(base) != product:
            raise ValueError("Release tag must match the committed product version")
        if git(root, "cat-file", "-t", ref) != "tag":
            raise ValueError("Release requires an annotated Git tag")
        if git(root, "rev-parse", f"{ref}^{{commit}}") != source:
            raise ValueError("Release tag does not identify HEAD")
        git(root, "merge-base", "--is-ancestor", source, "refs/remotes/origin/main")
        if dirty or not public:
            raise ValueError("Release requires a clean checkout and NBGV PublicRelease=true")
    display = tag[1:] if release else f"{product}-dev.{height}.g{source[:SHORT_LENGTH]}" + (".dirty" if dirty else "")
    informational = f"{display}+height.{height}.sha.{source}" + (".dirty" if dirty else "")
    return dict(schemaVersion=1, productVersion=product, displayVersion=display,
                informationalVersion=informational, sourceRevision=source, versionHeight=height,
                dirty=dirty, nbgvPublicRelease=public, releaseEligible=release and not development,
                developmentReleaseEligible=release and development, publicationEligible=release,
                identityStatus="resolved", toolIdentity=tool_identity())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["install", "resolve", "exec", "tag"])
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--event", default="local", choices=["local", "push", "pull_request", "workflow_dispatch"])
    parser.add_argument("--ref", default="")
    parser.add_argument("--sha")
    parser.add_argument("--dev", help="Create a numbered development tag (positive integer)")
    parser.add_argument("--fallback", action="store_true", help="Explicit non-publishing unknown identity")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--github-output", type=Path)
    args, command = parser.parse_known_args()
    if command and args.command != "exec":
        parser.error("Unexpected arguments: " + " ".join(command))
    if args.dev is not None and (args.command != "tag" or not re.fullmatch(r"[1-9][0-9]*", args.dev)):
        parser.error("--dev requires the tag command and a positive integer without leading zeroes")
    if args.command == "install":
        print(nbgv(install=True))
        return
    data = resolve(args.root.resolve(), args.event, args.ref, args.sha, args.fallback)
    if args.command == "tag":
        if data["dirty"] is not False or data["identityStatus"] != "resolved":
            raise ValueError("Tag creation requires a clean, resolved checkout")
        git(args.root, "merge-base", "--is-ancestor", data["sourceRevision"], "refs/remotes/origin/main")
        tag = "v" + data["productVersion"] + ("-dev." + args.dev if args.dev else "")
        signing = subprocess.run(["git", "-C", str(args.root), "config", "--bool", "tag.gpgSign"], capture_output=True, text=True)
        git(args.root, "tag", "-s" if signing.stdout.strip() == "true" else "-a", tag, "-m", f"Release {tag}")
        print(f"Created {tag}; not pushed")
    elif args.command == "exec":
        if command[:1] == ["--"]:
            command = command[1:]
        if not command:
            parser.error("exec requires -- COMMAND [ARG ...]")
        env = dict(os.environ, CITADEL_VERSION=data["displayVersion"],
                   CITADEL_INFORMATIONAL_VERSION=data["informationalVersion"],
                   CITADEL_SOURCE_REVISION=data["sourceRevision"] or "",
                   CITADEL_PRODUCT_VERSION=data["productVersion"])
        raise SystemExit(subprocess.run(command, env=env).returncode)
    else:
        output = json.dumps(data, indent=2) + "\n"
        if args.output:
            args.output.write_text(output)
        else:
            print(output, end="")
        if args.github_output:
            with args.github_output.open("a") as stream:
                for key, value in data.items():
                    stream.write(f"{key}={json.dumps(value) if not isinstance(value, str) else value}\n")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"Version resolution failed: {error}", file=sys.stderr)
        sys.exit(1)
