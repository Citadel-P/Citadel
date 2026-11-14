#!/usr/bin/env python3
"""VPS-side deployment, sent over SSH by demo.py. Requires Python 3.10+."""
import fcntl
import argparse
import contextlib
import types
import ipaddress
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
    if (set(target) not in ({"user", "uid", "dockerId"}, {"user", "uid", "dockerId", "network"}) or target["user"] != expected
            or account != expected or type(target["uid"]) is not int
            or target["uid"] != os.geteuid() or target["uid"] <= 0):
        raise ValueError("Rootless deployment account does not match target")
    if "network" in target:
        network = target["network"]
        if not isinstance(network, dict) or set(network) != {"subnet", "gateway"}:
            raise ValueError("Invalid rootless network configuration")
        subnet, gateway = ipaddress.IPv4Network(network["subnet"]), ipaddress.IPv4Address(network["gateway"])
        if (not subnet.is_private or subnet.is_loopback or gateway not in subnet
                or gateway in (subnet.network_address, subnet.broadcast_address)):
            raise ValueError("Invalid rootless network gateway")
    socket = f"unix:///run/user/{target['uid']}/docker.sock"
    info = json.loads(subprocess.check_output(
        ["docker", "--host", socket, "info", "--format", "{{json .}}"], text=True, timeout=30))
    if (not target["dockerId"] or info.get("ID") != target["dockerId"]
            or "name=rootless" not in info.get("SecurityOptions", [])
            or info.get("DockerRootDir") != f"/home/{expected}/.local/share/docker"):
        raise ValueError("Docker engine does not match the pinned rootless target")
    return target


def deployment_compose(source):
    # CI owns guarded registration/refresh for these installations. Keep that
    # explicit host-state flow; do not start a second first-install registrar.
    return source.replace('      CITADEL_UPDATE_STATE_DIRECTORY: /var/lib/citadel-updates\n', '').replace(
        '      - ./.citadel-updates:/var/lib/citadel-updates:ro\n', '')


def compose_override(channel, rootless=None):
    # Memory is unlimited by default. Keep CPU ceilings independently tunable
    # through operator-owned .env settings; CI changes image pins, not resource limits.
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
    # The host reverse proxy needs the optional Edge listener published.
    services["server"]["ports"] = [
        "${CITADEL_BIND_ADDRESS:-127.0.0.1}:${CITADEL_EDGE_PORT:-18001}:${CITADEL_EDGE_GRPC_LISTEN_PORT:-8001}",
    ]
    if rootless:
        # Replace the full mount list: never inherit the host socket or / mount.
        # Compose 2.24.4+ supports !override (required by operator provisioning).
        home = f"/home/{rootless['user']}"
        mounts = [f"/run/user/{rootless['uid']}/docker.sock:/var/run/docker.sock",
                  "citadel_data:/app/data",
                  f"{home}/.local/share/docker:/host{home}/.local/share/docker:ro"]
        services["server"]["volumes"] = "__ROOTLESS_MOUNTS__"
        override = {"services": services}
        if "network" in rootless:
            # Preserve the bridge gateway already trusted by the reverse proxy
            # configuration in the operator-owned .env, including on recreation.
            override["networks"] = {"default": {"ipam": {"config": [rootless["network"]]}}}
        return json.dumps(override, indent=2).replace(
            '"__ROOTLESS_MOUNTS__"', "!override " + json.dumps(mounts)) + "\n"
    return json.dumps({"services": services}, indent=2) + "\n"


def load_update_setup(source):
    if not isinstance(source, str) or not source.strip():
        raise ValueError("Missing release update setup tool")
    setup = types.ModuleType("citadel_update_setup")
    exec(compile(source, "release-enable-updates.py", "exec"), setup.__dict__)
    return setup


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
        docker = ["docker", "--host", "unix:///var/run/docker.sock"] if rootless is None else ["docker", "--host", f"unix:///run/user/{rootless['uid']}/docker.sock"]
        setup = load_update_setup(payload.get("update_setup"))
        # Same trusted checkout as the remote deploy program; never fetch a script
        # from a moving branch on the VPS. Its output must not corrupt our JSON.
        _, settings = setup.read_environment(directory / ".env")
        args = argparse.Namespace(directory=str(directory), project=settings.get("COMPOSE_PROJECT_NAME"),
                                  compose_file=[str(directory / "docker-compose.yml"), str(directory / "demo.override.yml")],
                                  state_directory=None, start=False)
        if rootless and setup.SETTING not in settings:
            sibling = directory.with_name(directory.name + '-update-state')
            # Rootless deployments may own their installation but not /opt.
            # Never bypass existing state, including an interrupted registration.
            if not sibling.exists() and not sibling.is_symlink():
                args.state_directory = f"/home/{rootless['user']}/{directory.name}-update-state"
        compose = [*docker, "compose", "--env-file", ".env",
                   "-f", "docker-compose.yml", "-f", "demo.override.yml"]
        env = {key: value for key, value in os.environ.items() if key in (
            "PATH", "HOME", "LANG", "LC_ALL", "DOCKER_CONFIG", "XDG_RUNTIME_DIR")}
        def run(*args, timeout):
            subprocess.run([*compose, *args], cwd=directory, env=env, check=True,
                           stdout=sys.stderr, timeout=timeout)
        def replace():
            original, _ = setup.read_environment(directory / ".env")
            database = ""
            if (directory / "docker-compose.yml").exists():
                database = subprocess.check_output([*compose, "ps", "--quiet", "pg_db"],
                                                   cwd=directory, env=dict(env, CITADEL_IMAGE=payload["core"],
                                                                           CITADEL_EDGE_AGENT_IMAGE=payload["agent"]),
                                                   text=True, timeout=30).strip()
            pins = {"CITADEL_IMAGE": payload["core"], "CITADEL_EDGE_AGENT_IMAGE": payload["agent"]}
            if database:
                # Keep the currently deployed database image when the shipped
                # template advances its default. Only reviewed DB updates change it.
                pins["PG_IMAGE"] = subprocess.check_output([*docker, "inspect", "--format", "{{.Config.Image}}", database],
                                                         text=True, timeout=30).strip()
                if not re.fullmatch(r"postgres@sha256:[a-f0-9]{64}", pins["PG_IMAGE"]):
                    raise ValueError("The existing database must use a pinned PostgreSQL image")
            lines = [line for line in original.decode().splitlines()
                     if line.strip().partition("=")[0] not in pins]
            contents = "\n".join(lines) + "\n" + "".join(f"{key}={value}\n" for key, value in pins.items())
            if setup.read_environment(directory / ".env")[0] != original:
                raise ValueError("Installation environment changed during deployment")
            atomic(directory / ".env", contents)
            atomic(directory / "docker-compose.yml", deployment_compose(payload["compose"]))
            atomic(directory / "demo.override.yml", compose_override(payload["channel"], rootless))
            atomic(directory / ".release.env", f"CITADEL_IMAGE={payload['core']}\nCITADEL_EDGE_AGENT_IMAGE={payload['agent']}\n")
            run("config", "--quiet", timeout=30)
            if setup.SETTING not in settings:
                run("pull", timeout=600)
            atomic(attempted, json.dumps(identity, indent=2) + "\n")
            if database:
                run("up", "--detach", "--no-deps", "--wait", "--wait-timeout", "180", "server", timeout=240)
            else:
                run("up", "--detach", "--wait", "--wait-timeout", "180", timeout=240)
        with contextlib.redirect_stdout(sys.stderr), setup.locked_installation(directory):
            # Pin the same daemon for setup, independent of the caller's context.
            old_context, old_host = os.environ.pop("DOCKER_CONTEXT", None), os.environ.get("DOCKER_HOST")
            if rootless is not None:
                os.environ["DOCKER_HOST"] = f"unix:///run/user/{rootless['uid']}/docker.sock"
            else:
                os.environ["DOCKER_HOST"] = "unix:///var/run/docker.sock"
            try:
                state = setup.resolve_state_directory(directory, settings, args.state_directory)
                setup.prepare_state_directory(state)
                if setup.SETTING in settings:
                    # Download before stopping Core, so registry latency does not
                    # extend its replacement downtime.
                    subprocess.run([*docker, "pull", payload["core"]], check=True,
                                   stdout=sys.stderr, timeout=600)
                    setup.enable(args, replace=replace)
                else:
                    replace()
                    setup.enable(args)
            except setup.SetupError as error:
                raise ValueError(str(error)) from None
            finally:
                if old_context is not None:
                    os.environ["DOCKER_CONTEXT"] = old_context
                if old_host is None:
                    os.environ.pop("DOCKER_HOST", None)
                else:
                    os.environ["DOCKER_HOST"] = old_host
        # This records local Compose health; demo.py verifies public HTTPS next.
        atomic(directory / "deployment.json", json.dumps(identity, indent=2) + "\n")
        return {"status": "deployed", **identity}


if __name__ == "__main__":
    try:
        print(json.dumps(deploy(Path(sys.argv[1]), json.load(sys.stdin))))
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Demo deployment failed: {error}", file=sys.stderr)
        sys.exit(1)
