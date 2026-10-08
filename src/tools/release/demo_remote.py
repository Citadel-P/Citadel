#!/usr/bin/env python3
"""VPS-side deployment, sent over SSH by demo.py. Requires Python 3.10+."""
import fcntl
import json
import os
from pathlib import Path
import pwd
import re
import subprocess
import sys
import time


IDENTITY = ("channel", "version", "core", "agent")


def order(value, channel):
    if channel not in ("dev", "latest"):
        raise ValueError("Expected deployment channel dev or latest")
    pattern = r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    if channel == "dev":
        pattern += r"-dev\.([1-9][0-9]*)"
    match = re.fullmatch(pattern, value)
    if not match:
        raise ValueError(f"Invalid version for deployment channel {channel}")
    return tuple(map(int, match.groups()))


def validate(payload):
    order(payload["version"], payload["channel"])
    for component, repository in [("core", "citadel"), ("agent", "citadel.agent")]:
        if not re.fullmatch(r"ghcr\.io/citadel-p/" + re.escape(repository) + r"@sha256:[a-f0-9]{64}", payload[component]):
            raise ValueError("Demo requires recorded GHCR image digests")


def atomic(path, data):
    temporary = path.with_name(path.name + ".pending")
    temporary.write_text(data)
    temporary.replace(path)


def acquire_lock(lock, timeout=120):
    deadline = time.monotonic() + timeout
    while True:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            return
        except BlockingIOError:
            if time.monotonic() >= deadline:
                raise TimeoutError("Timed out waiting for the deployment lock")
            time.sleep(1)


def deployment_state(directory, channel):
    """Validate all history before pinning or normalizing legacy dev records."""
    guard = directory / ".deployment-channel"
    pinned = guard.read_text().removesuffix("\n") if guard.exists() else None
    if pinned is not None and pinned not in ("dev", "latest"):
        raise ValueError("Invalid .deployment-channel content")
    records = {}
    for name in ("deployment-attempt.json", "deployment.json"):
        path = directory / name
        if path.exists():
            previous = json.loads(path.read_text())
            if not isinstance(previous, dict) or set(previous) not in (set(IDENTITY), set(IDENTITY) - {"channel"}):
                raise ValueError(f"Invalid deployment identity in {name}")
            previous = {"channel": "dev", **previous}
            validate(previous)
            if pinned is None:
                pinned = previous["channel"]
            if previous["channel"] != pinned:
                raise ValueError("Deployment history and target channel disagree")
            records[name] = previous
    pinned = pinned or channel
    if pinned != channel:
        raise ValueError(f"Deployment target is pinned to channel '{pinned}'; received '{channel}'")
    attempted, completed = records.get("deployment-attempt.json"), records.get("deployment.json")
    if completed:
        if not attempted or order(completed["version"], channel) > order(attempted["version"], channel):
            raise ValueError("Completed deployment has no matching or newer attempt history")
        if completed["version"] == attempted["version"] and completed != attempted:
            raise ValueError("Deployment history has different image digests for the same version")
    # All records are valid. Preserve legacy high-water marks while adding channel.
    if not guard.exists():
        atomic(guard, pinned + "\n")
    for name, previous in records.items():
        path = directory / name
        if "channel" not in json.loads(path.read_text()):
            atomic(path, json.dumps(previous, indent=2) + "\n")
    return attempted


def rootless_target(directory, channel):
    """Bind an operator-provisioned target to its account and Docker engine."""
    marker = directory / ".rootless-deployment.json"
    account = pwd.getpwuid(os.geteuid()).pw_name
    if not marker.exists():
        if account in ("citadel-preview", "citadel-demo"):
            raise ValueError("Missing rootless deployment configuration")
        return None
    target = json.loads(marker.read_text())
    expected = "citadel-preview" if channel == "dev" else "citadel-demo"
    if (set(target) != {"user", "uid", "dockerId"} or target["user"] != expected
            or account != expected or type(target["uid"]) is not int
            or target["uid"] != os.geteuid() or target["uid"] <= 0):
        raise ValueError("Rootless deployment account does not match target")
    socket = f"unix:///run/user/{target['uid']}/docker.sock"
    info = json.loads(subprocess.check_output(
        ["docker", "--host", socket, "info", "--format", "{{json .}}"], text=True, timeout=30))
    if (not target["dockerId"] or info.get("ID") != target["dockerId"]
            or "name=rootless" not in info.get("SecurityOptions", [])
            or info.get("DockerRootDir") != f"/home/{expected}/.local/share/docker"):
        raise ValueError("Docker engine does not match the pinned rootless target")
    return target


def compose_override(channel, rootless=None):
    # Memory is unlimited by default. Keep CPU ceilings independently tunable
    # through the operator-owned .env without CI rewriting configuration.
    core_cpus, database_cpus = ("1.0", "0.5") if channel == "dev" else ("2.0", "1.0")
    services = {}
    for name, prefix, cpus in (
        ("server", "CORE", core_cpus),
        ("pg_db", "DATABASE", database_cpus),
    ):
        limit = "${CITADEL_" + prefix + "_MEMORY_LIMIT:-0}"
        services[name] = {"mem_limit": limit, "memswap_limit": limit,
                          "cpus": "${CITADEL_" + prefix + "_CPUS:-" + cpus + "}"}
    services["server"]["environment"] = {
        "CITADEL_EDGE_AGENT_IMAGE": "${CITADEL_EDGE_AGENT_IMAGE:?Missing Agent digest}",
    }
    if rootless:
        # Replace the full mount list: never inherit the host socket or / mount.
        # Compose 2.24.4+ supports !override (required by operator provisioning).
        home = f"/home/{rootless['user']}"
        mounts = [f"/run/user/{rootless['uid']}/docker.sock:/var/run/docker.sock",
                  "citadel_data:/app/data",
                  f"{home}/.local/share/docker:/host{home}/.local/share/docker:ro"]
        services["server"]["volumes"] = "__ROOTLESS_MOUNTS__"
        return json.dumps({"services": services}, indent=2).replace(
            '"__ROOTLESS_MOUNTS__"', "!override " + json.dumps(mounts)) + "\n"
    return json.dumps({"services": services}, indent=2) + "\n"


def deploy(directory, payload):
    validate(payload)
    if not isinstance(payload.get("compose"), str) or not payload["compose"].strip():
        raise ValueError("Missing release Compose file")
    if not directory.is_absolute() or directory.resolve() == Path("/") or not (directory / ".env").is_file():
        raise ValueError("Provision a dedicated absolute deployment directory and .env first")
    os.umask(0o077)
    with (directory / ".deployment.lock").open("a") as lock:
        acquire_lock(lock)
        rootless = rootless_target(directory, payload["channel"])
        previous = deployment_state(directory, payload["channel"])
        attempted = directory / "deployment-attempt.json"
        identity = {key: payload[key] for key in IDENTITY}
        if previous:
            if order(previous["version"], payload["channel"]) > order(payload["version"], payload["channel"]):
                return {"status": "skipped-older-release", **identity}
            if previous["version"] == payload["version"] and previous != identity:
                raise ValueError("Existing version has different image digests")
        docker = ["docker"] if rootless is None else ["docker", "--host", f"unix:///run/user/{rootless['uid']}/docker.sock"]
        compose = [*docker, "compose", "--env-file", ".env", "--env-file", ".release.env",
                   "-f", "docker-compose.yml", "-f", "demo.override.yml"]
        env = dict(os.environ, CITADEL_IMAGE=payload["core"], CITADEL_EDGE_AGENT_IMAGE=payload["agent"])
        def run(*args, timeout):
            subprocess.run([*compose, *args], cwd=directory, env=env, check=True,
                           stdout=sys.stderr, timeout=timeout)
        atomic(directory / "docker-compose.yml", payload["compose"])
        atomic(directory / "demo.override.yml", compose_override(payload["channel"], rootless))
        atomic(directory / ".release.env", f"CITADEL_IMAGE={payload['core']}\nCITADEL_EDGE_AGENT_IMAGE={payload['agent']}\n")
        run("config", "--quiet", timeout=30)
        run("pull", timeout=600)
        # Persist the highest attempted version before migrations can run. An
        # older workflow cannot silently roll back a partially upgraded database.
        atomic(attempted, json.dumps(identity, indent=2) + "\n")
        run("up", "--detach", "--wait", "--wait-timeout", "180", timeout=240)
        # This records local Compose health; demo.py verifies public HTTPS next.
        atomic(directory / "deployment.json", json.dumps(identity, indent=2) + "\n")
        return {"status": "deployed", **identity}


if __name__ == "__main__":
    try:
        print(json.dumps(deploy(Path(sys.argv[1]), json.load(sys.stdin))))
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Demo deployment failed: {error}", file=sys.stderr)
        sys.exit(1)
