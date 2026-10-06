#!/usr/bin/env python3
"""Deploy completed dev-release digests to a provisioned VPS; dry run by default."""
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


def deployment_payload(metadata, backend):
    release.check_metadata(metadata)
    if not release.development(metadata):
        raise ValueError("Demo only accepts numbered development releases")
    tag = "v" + metadata["displayVersion"]
    record = backend.load(tag)
    published = backend.release(tag)
    if (not record or record["metadata"] != metadata or record["status"] != "complete"
            or not published or published["draft"] or not published.get("prerelease")):
        raise ValueError("Demo requires a completed, published development release")
    payload = {"version": metadata["displayVersion"]}
    for component, repositories in release.repositories(metadata).items():
        repository = repositories[0]
        digest = record["indexes"][component]
        if (not re.fullmatch(r"sha256:[a-f0-9]{64}", digest)
                or record["steps"].get("signed:" + repository + ":" + metadata["displayVersion"]) != digest):
            raise ValueError("Missing verified signature checkpoint")
        payload[component] = repository + "@" + digest
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
    for attempt in range(12):
        try:
            with urllib.request.urlopen(url + "/health", timeout=10) as response:
                if response.status == 200 and json.load(response).get("status") == "ok":
                    return
        except (OSError, ValueError, urllib.error.URLError):
            pass
        if attempt < 11:
            time.sleep(5)
    raise ValueError("Demo HTTPS health check failed; inspect the deployment before retrying")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--identity", type=Path, required=True)
    parser.add_argument("--known-hosts", type=Path, required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    metadata = json.loads(args.metadata.read_text())
    if args.apply and (os.environ.get("GITHUB_EVENT_NAME") != "push"
                       or os.environ.get("GITHUB_REF") != "refs/tags/v" + metadata["displayVersion"]
                       or os.environ.get("GITHUB_SHA") != metadata["sourceRevision"]):
        raise ValueError("Demo writes require the original development-tag push context")
    if args.repository.lower() != "citadel-p/citadel":
        raise ValueError("Demo releases must come from Citadel-P/Citadel")
    payload = deployment_payload(metadata, release.Backend(args.repository))
    host, user, port, directory, url = settings(os.environ)
    if not args.apply:
        print(json.dumps({"dryRun": True, "host": host, "directory": directory,
                          **{k: v for k, v in payload.items() if k != "compose"}}, indent=2))
        return
    remote = Path(__file__).with_name("demo_remote.py").read_text()
    result = subprocess.run([
        "ssh", "-T", "-p", port, "-i", str(args.identity), "-o", "BatchMode=yes",
        "-o", "IdentitiesOnly=yes", "-o", "StrictHostKeyChecking=yes",
        "-o", f"UserKnownHostsFile={args.known_hosts}", "-o", "ConnectTimeout=15",
        "-o", "ServerAliveInterval=15", "-o", "ServerAliveCountMax=3", f"{user}@{host}",
        "python3 -c " + shlex.quote(remote) + " " + shlex.quote(directory),
    ], input=json.dumps(payload), text=True, stdout=subprocess.PIPE, check=True, timeout=780)
    outcome = json.loads(result.stdout)
    if outcome["status"] == "deployed":
        verify_health(url)
    print(json.dumps(outcome, indent=2))
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
            summary.write(f"Demo {payload['version']}: {outcome['status']} — {url}\n")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
        print(f"Demo deployment blocked: {error}", file=sys.stderr)
        sys.exit(1)
