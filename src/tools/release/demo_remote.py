#!/usr/bin/env python3
"""VPS-side deployment, sent over SSH by demo.py. Requires Python 3.10+."""
import fcntl
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def order(value):
    match = re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)-dev\.([1-9][0-9]*)", value)
    if not match:
        raise ValueError("Expected a numbered development release")
    return tuple(map(int, match.groups()))


def validate(payload):
    order(payload["version"])
    for component, repository in [("core", "citadel"), ("agent", "citadel.agent")]:
        if not re.fullmatch(r"ghcr\.io/citadel-p/" + re.escape(repository) + r"@sha256:[a-f0-9]{64}", payload[component]):
            raise ValueError("Demo requires recorded GHCR image digests")


def atomic(path, data):
    temporary = path.with_name(path.name + ".pending")
    temporary.write_text(data)
    temporary.replace(path)


def deploy(directory, payload):
    validate(payload)
    if not directory.is_absolute() or directory.resolve() == Path("/") or not (directory / ".env").is_file():
        raise ValueError("Provision a dedicated absolute deployment directory and .env first")
    os.umask(0o077)
    with (directory / ".deployment.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        attempted = directory / "deployment-attempt.json"
        identity = {key: payload[key] for key in ["version", "core", "agent"]}
        if attempted.exists():
            previous = json.loads(attempted.read_text())
            if order(previous["version"]) > order(payload["version"]):
                return {"status": "skipped-older-release", "version": payload["version"]}
            if previous["version"] == payload["version"] and previous != identity:
                raise ValueError("Existing development version has different image digests")
        compose = ["docker", "compose", "--env-file", ".env", "--env-file", ".release.env",
                   "-f", "docker-compose.yml", "-f", "demo.override.yml"]
        env = dict(os.environ, CITADEL_IMAGE=payload["core"], CITADEL_EDGE_AGENT_IMAGE=payload["agent"])
        def run(*args):
            subprocess.run([*compose, *args], cwd=directory, env=env, check=True,
                           stdout=sys.stderr, timeout=600)
        atomic(directory / "docker-compose.yml", payload["compose"])
        atomic(directory / "demo.override.yml", "services:\n  server:\n    environment:\n      CITADEL_EDGE_AGENT_IMAGE: ${CITADEL_EDGE_AGENT_IMAGE:?Missing Agent digest}\n")
        atomic(directory / ".release.env", f"CITADEL_IMAGE={payload['core']}\nCITADEL_EDGE_AGENT_IMAGE={payload['agent']}\n")
        run("config", "--quiet")
        run("pull")
        # Persist the highest attempted version before migrations can run. An
        # older workflow cannot silently roll back a partially upgraded database.
        atomic(attempted, json.dumps(identity, indent=2) + "\n")
        run("up", "--detach", "--wait", "--wait-timeout", "180")
        atomic(directory / "deployment.json", json.dumps(identity, indent=2) + "\n")
        return {"status": "deployed", **identity}


if __name__ == "__main__":
    try:
        print(json.dumps(deploy(Path(sys.argv[1]), json.load(sys.stdin))))
    except (ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
        print(f"Demo deployment failed: {error}", file=sys.stderr)
        sys.exit(1)
