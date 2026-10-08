#!/usr/bin/env python3
"""Deploy completed dev/latest release digests to a provisioned VPS; dry run by default."""
import argparse
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

import release


def deployment_payload(metadata, backend, channel):
    if channel not in ("dev", "latest"):
        raise ValueError("Expected deployment channel dev or latest")
    release.check_metadata(metadata)
    if (channel == "dev" and not release.development(metadata)
            or channel == "latest" and not metadata["releaseEligible"]):
        raise ValueError("Release metadata does not match the deployment channel")
    tag = "v" + metadata["displayVersion"]
    record = backend.load(tag)
    published = backend.release(tag)
    if (not record or record["metadata"] != metadata or record["status"] != "complete"
            or not published or published["draft"]
            or published.get("prerelease") is not (channel == "dev")):
        raise ValueError("Demo requires a completed, published release for the selected channel")
    payload = {"channel": channel, "version": metadata["displayVersion"]}
    for component, repositories in release.repositories(metadata).items():
        repository = repositories[0]
        digest = record["indexes"][component]
        if (not re.fullmatch(r"sha256:[a-f0-9]{64}", digest)
                or record["steps"].get("signed:" + repository + ":" + metadata["displayVersion"]) != digest):
            raise ValueError("Missing verified signature checkpoint")
        payload[component] = repository + "@" + digest
    # Publication and this job share a concurrency group. Recheck ownership only
    # after acquiring that slot; publication alone controls mutable aliases.
    stale = False
    for component in (("core", "agent") if channel == "dev" else ("core",)):
        repository, expected = payload[component].split("@")
        actual = backend.existing(repository, channel)
        if not isinstance(actual, str) or not re.fullmatch(r"sha256:[a-f0-9]{64}", actual):
            raise ValueError(f"Missing or invalid {component} {channel} alias")
        stale |= actual != expected
    if stale:
        return {"status": "skipped-stale-alias", **payload}
    payload["compose"] = (release.ROOT / "deploy/install/docker-compose.yml").read_text()
    return payload


def settings(env):
    host, user = env.get("CITADEL_DEMO_HOST", ""), env.get("CITADEL_DEMO_USER", "")
    port = env.get("CITADEL_DEMO_PORT", "22")
    directory, url = env.get("CITADEL_DEMO_DIRECTORY", ""), env.get("CITADEL_DEMO_URL", "")
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.-]*", host) or not re.fullmatch(r"[a-z_][a-z0-9_-]*", user):
        raise ValueError("Configure the demo SSH hostname and username")
    if not port.isascii() or not port.isdigit() or not 1 <= int(port) <= 65535:
        raise ValueError("Invalid demo SSH port")
    if not directory.startswith("/") or directory == "/" or any(c in directory for c in "\n\r\0"):
        raise ValueError("Configure a dedicated absolute demo directory")
    parsed = urllib.parse.urlsplit(url)
    if (parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password
            or parsed.path not in ("", "/") or parsed.query or parsed.fragment):
        raise ValueError("CITADEL_DEMO_URL must be an HTTPS origin, e.g. https://demo.example.com")
    return host, user, port, directory, url.rstrip("/")


def verify_health(url):
    request = urllib.request.Request(url + "/health", headers={
        "User-Agent": "Citadel-Release-Healthcheck/1.0", "Accept": "application/json",
    })
    for attempt in range(12):
        try:
            with urllib.request.urlopen(request, timeout=10) as response:
                data = json.load(response)
                if response.status == 200 and isinstance(data, dict) and data.get("status") == "ok":
                    return
        except (OSError, ValueError, urllib.error.URLError):
            pass
        if attempt < 11:
            time.sleep(5)
    raise ValueError("Demo HTTPS health check failed; inspect the deployment before retrying")


def write_summary(identity, status, url):
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if path:
        with open(path, "a") as summary:
            summary.write("### Citadel deployment\n\n")
            for label, value in [("Channel", identity.get("channel", "unavailable")),
                                 ("Version", identity.get("version", "unavailable")),
                                 ("Core", identity.get("core", "unavailable")),
                                 ("Agent", identity.get("agent", "unavailable")),
                                 ("Result", status), ("URL", url or "unavailable")]:
                summary.write(f"- {label}: {value}\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--channel", choices=("dev", "latest"), required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--identity", type=Path, required=True)
    parser.add_argument("--known-hosts", type=Path, required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    payload = {"channel": args.channel}
    status, url = "failed-preflight", ""
    try:
        metadata = json.loads(args.metadata.read_text())
        payload["version"] = metadata["displayVersion"]
        host, user, port, directory, url = settings(os.environ)
        if args.apply:
            if (os.environ.get("GITHUB_EVENT_NAME") != "push"
                    or os.environ.get("GITHUB_REF") != "refs/tags/v" + metadata["displayVersion"]
                    or os.environ.get("GITHUB_SHA") != metadata["sourceRevision"]):
                raise ValueError("Demo writes require the original release-tag push context")
            head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=release.ROOT, text=True).strip()
            if head != metadata["sourceRevision"]:
                raise ValueError("Deployment checkout does not match the released source revision")
        if args.repository.lower() != "citadel-p/citadel":
            raise ValueError("Demo releases must come from Citadel-P/Citadel")
        payload = deployment_payload(metadata, release.Backend(args.repository), args.channel)
        if payload.get("status") == "skipped-stale-alias":
            status = payload["status"]
            print(json.dumps(payload, indent=2))
            return
        if not args.apply:
            status = "dry-run"
            print(json.dumps({"dryRun": True, "host": host, "directory": directory,
                              **{k: v for k, v in payload.items() if k != "compose"}}, indent=2))
            return
        status = "failed-remote-deployment"
        remote = Path(__file__).with_name("demo_remote.py").read_text()
        result = subprocess.run([
            "ssh", "-T", "-p", port, "-i", str(args.identity), "-o", "BatchMode=yes",
            "-o", "IdentitiesOnly=yes", "-o", "StrictHostKeyChecking=yes",
            "-o", f"UserKnownHostsFile={args.known_hosts}", "-o", "ConnectTimeout=15",
            "-o", "ServerAliveInterval=15", "-o", "ServerAliveCountMax=3", f"{user}@{host}",
            "python3 -c " + shlex.quote(remote) + " " + shlex.quote(directory),
        ], input=json.dumps(payload), text=True, stdout=subprocess.PIPE, check=True, timeout=1080)
        outcome = json.loads(result.stdout)
        identity = {key: payload[key] for key in ("channel", "version", "core", "agent")}
        if (outcome.get("status") not in ("deployed", "skipped-older-release")
                or any(outcome.get(key) != value for key, value in identity.items())):
            raise ValueError("Unexpected remote deployment result")
        if outcome["status"] == "deployed":
            status = "failed-public-health"
            verify_health(url)
        status = outcome["status"]
        print(json.dumps(outcome, indent=2))
    finally:
        write_summary(payload, status, url)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Demo deployment blocked: {error}", file=sys.stderr)
        sys.exit(1)
