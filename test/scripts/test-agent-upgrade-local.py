#!/usr/bin/env python3
"""Real Direct/Edge Agent updates in disposable Docker daemons, without publication.

Use --browser to leave review/approval to the user; otherwise verify both rollouts.
Only the test Core substitutes the release catalog. Agents use production binaries.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time
import uuid

spec = importlib.util.spec_from_file_location(
    "local_upgrade", Path(__file__).with_name("test-instance-upgrade-local.py"))
local = importlib.util.module_from_spec(spec)
spec.loader.exec_module(local)
registration = local.registration
require = local.require


def mount_identity(container):
    # Docker may normalize Mode from "" to "rw,rprivate" during recreation.
    # Compare actual storage and effective access/propagation, independent of order.
    identities = []
    for mount in container["Mounts"]:
        options = set(filter(None, mount.get("Mode", "").split(",")))
        options -= {"rw", "ro", "private", "rprivate", "shared", "rshared", "slave", "rslave"}
        identities.append(tuple(mount.get(key) for key in
            ("Type", "Source", "Name", "Destination", "RW", "Propagation")) + (tuple(sorted(options)),))
    return sorted(identities)


def published_ports(container):
    # Compare bound endpoints; Docker expands an empty requested HostIp to IPv4/IPv6.
    return sorted((port, binding["HostIp"], binding["HostPort"])
                  for port, bindings in (container["NetworkSettings"].get("Ports") or {}).items()
                  for binding in (bindings or []))


def build_agent(base, version, directory):
    print(f"Building private Agent fixture {version}", flush=True)
    result = subprocess.run([
        "cargo", "build", "--locked", "-p", "citadel-agent", "--bin", "citadel-agent",
        "--message-format=json",
    ], cwd=registration.ROOT, env=dict(os.environ, CITADEL_VERSION=version,
        CITADEL_INFORMATIONAL_VERSION=version, SQLX_OFFLINE="true"),
        capture_output=True, timeout=1800)
    if result.returncode:
        log = Path(tempfile.mkdtemp(prefix="citadel-agent-build-failure-")) / "build.log"
        log.write_bytes(result.stdout + result.stderr)
        raise RuntimeError(f"Agent compilation failed; see {log}")
    artifacts = [json.loads(line) for line in result.stdout.splitlines() if line.startswith(b"{")]
    executable = next(item["executable"] for item in artifacts
                      if item.get("reason") == "compiler-artifact"
                      and item["target"]["name"] == "citadel-agent" and item.get("executable"))
    context = directory / ("agent-" + version)
    context.mkdir()
    shutil.copy2(executable, context / "citadel-agent")
    (context / "Dockerfile").write_text(
        f"FROM {base}\nCOPY --chmod=755 citadel-agent /app/citadel-agent\n"
        "LABEL com.citadel.qualification-only=true\n")
    image = f"citadel-agent-upgrade-fixture:{version}-{directory.name}"
    registration.run([*registration.DOCKER, "build", "--builder", "default",
                      "--tag", image, str(context)], timeout=300)
    return image


class AgentUpgrade(local.LocalUpgrade):
    extra_environment = (
        "CITADEL_HTTP_ALLOWED_HOSTS=localhost;127.0.0.1;fixture-core\n"
        "CITADEL_EDGE_PUBLIC_GRPC_URL=http://fixture-core:8001\n"
    )

    def docker(self, *args, **kwargs):
        check = kwargs.pop("check", True)
        result = super().docker(*args, check=False, **kwargs)
        if check and result.returncode:
            directory = Path(tempfile.mkdtemp(prefix="citadel-agent-command-failure-"))
            log = directory / "command.log"
            log.write_bytes(result.stdout + result.stderr)
            raise RuntimeError(f"Fixture Docker command failed; private output: {log}")
        return result

    def capture_failure(self):
        super().capture_failure()
        destination = Path(tempfile.mkdtemp(prefix="citadel-agent-upgrade-failure-"))
        for kind in ("direct", "edge"):
            containers = self.nested(kind, "ps", "-aq", check=False).stdout.decode().split()
            for container in containers:
                result = self.nested(kind, "logs", "--tail=500", container, check=False)
                (destination / (kind + "-" + container + ".log")).write_bytes(result.stdout + result.stderr)
                inspection = self.nested(kind, "inspect", container, check=False)
                (destination / (kind + "-" + container + ".json")).write_bytes(inspection.stdout)
        print(f"Private Agent logs retained at {destination}", flush=True)

    def prepare_agents(self, source, target):
        # The existing runner creates the private registry and packages the Core
        # fixture. Core stays on TARGET; only the two Agents start on SOURCE.
        self.prepare_images(self.core)
        self.load(source)
        self.load(target)
        self.load(registration.DIND)
        # save/load preserves image IDs, but may drop repository digests.
        self.dind_image = self.host("image", "inspect", registration.DIND,
                                    "--format", "{{.Id}}").stdout.decode().strip()
        repository = "ghcr.io/citadel-p/citadel.agent"
        self.agent_source = self.publish_private(source, "source", repository)
        self.agent_target = self.publish_private(target, "target", repository)
        config = json.loads(self.docker("run", "--rm", "--network=none", "--entrypoint=cat",
                                       self.core, "/app/upgrade-fixture.json").stdout)
        config["agent"] = dict(version=local.TARGET, image=self.agent_target)
        self.write("/opt/fixture-build/upgrade-fixture.json", json.dumps(config))
        self.write("/opt/fixture-build/Dockerfile",
                   f"FROM {self.core}\nCOPY --chmod=644 upgrade-fixture.json /app/upgrade-fixture.json\n"
                   "ENV CITADEL_CORE_AGENT_ALLOW_INSECURE=true\n")
        self.docker("build", "--tag", "citadel-agent-fixture-core", "/opt/fixture-build", timeout=300)
        self.core = self.publish_private("citadel-agent-fixture-core", "agents-core")

    @staticmethod
    def socket(kind):
        return "/run/user/1001/docker.sock" if kind == "edge" else "/var/run/docker.sock"

    def nested(self, kind, *args, **kwargs):
        return self.docker("exec", "fixture-" + kind + "-docker", "docker",
                           "--host", "unix://" + self.socket(kind), *args, **kwargs)

    def wait_for(self, probe, message, seconds=180):
        deadline = time.monotonic() + seconds
        while True:
            result = probe()
            if result:
                return result
            require(time.monotonic() < deadline, message)
            time.sleep(1)

    def setup_agents(self, browser_port=None):
        registered, _, password = self.setup_installation(browser_port)
        token = self.login(password)
        core = self.json_command("inspect", registered["coreContainerId"])[0]
        self.core_before = local.upgrade.database_state(core)
        network, address = next(iter(core["NetworkSettings"]["Networks"].items()))
        public_key = self.http("platforms/agent/setup", token=token)["hubPublicKey"]
        self.agent_before = {}
        for kind in ("direct", "edge"):
            print(f"Starting isolated {kind.title()} Agent", flush=True)
            # Edge exercises the rootless socket layout with no standard socket fallback.
            # This nested daemon still runs as root; it does not qualify rootless kernel setup.
            # No Docker socket or TCP endpoint is exposed on the physical host.
            self.docker("run", "-d", "--privileged", "--name", f"fixture-{kind}-docker",
                        "--network", network, "--add-host", "ghcr.io:" + address["Gateway"],
                        "-e", "DOCKER_TLS_CERTDIR=", "--entrypoint", "sh", self.dind_image,
                        "-c", f"mkdir -p {Path(self.socket(kind)).parent}; "
                        f"exec dockerd-entrypoint.sh dockerd --host=unix://{self.socket(kind)} --insecure-registry=ghcr.io")
            self.wait_for(lambda: self.nested(kind, "info", check=False).returncode == 0,
                          kind + " Docker daemon did not start")
            if kind == "edge":
                require(self.docker("exec", "fixture-edge-docker", "test", "!", "-S", "/var/run/docker.sock", check=False).returncode == 0,
                        "Edge fixture unexpectedly exposes the standard Docker socket")
            self.nested(kind, "pull", self.agent_source, timeout=300)
            options = ["run", "-d", "--name", "citadel-agent", "--restart=unless-stopped",
                       "-v", self.socket(kind) + ":/var/run/docker.sock",
                       "-v", "agent-data:/app/data", "--health-interval=1s"]
            if kind == "direct":
                agent_id = str(uuid.uuid4())
                self.nested(kind, *options, "-p", "9000:9000",
                            "-e", "CITADEL_CORE_PUBLIC_KEY=" + public_key,
                            "-e", "CITADEL_AGENT_ID=" + agent_id, self.agent_source)
                self.wait_for(lambda: self.nested(kind, "exec", "citadel-agent",
                    "/app/citadel-agent", "healthcheck", check=False).returncode == 0,
                    "Direct Agent did not start")
                platform = self.http("platforms", dict(name="fixture-direct-agent", type="Docker",
                    connectorType="Agent", address="http://fixture-direct-docker:9000",
                    agentId=agent_id, tagIds=[]), token)
            else:
                self.nested(kind, "network", "create", "fixture-edge_default")
                self.nested(kind, "network", "create", "fixture-monitoring")
                options += ["--network", "fixture-edge_default", "--network-alias", "agent"]
                platform = self.http("platforms", dict(name="fixture-edge-agent", type="Docker",
                    connectorType="EdgeAgent", tagIds=[]), token)
                enrollment = self.http(f"platforms/{platform['id']}/edge/enrollments", {}, token)
                self.nested(kind, *options, "--add-host", "fixture-core:" + address["IPAddress"],
                    "--label", "com.citadel.system-role=edge-agent",
                    "-e", "CITADEL_AGENT_MODE=EdGe",
                    "-e", "CITADEL_CORE_URL=http://fixture-core:8001",
                    "-e", "CITADEL_EDGE_ENROLLMENT_TOKEN=" + enrollment["token"], self.agent_source)
                self.wait_for(lambda: self.http(f"platforms/{platform['id']}/edge/status",
                    token=token)["connectionStatus"] == "Connected", "Edge Agent did not connect")
            if kind == "edge":
                self.nested(kind, "network", "connect", "--gw-priority=1", "--alias", "monitor", "fixture-monitoring", "citadel-agent")
            self.agent_before[kind] = json.loads(self.nested(kind, "inspect", "citadel-agent").stdout)[0]
        self.http("updates/check", {}, token)
        targets = self.http("updates/agents", token=token)["targets"]
        require(len(targets) == 2 and all(t["status"] == "Available"
                and t["installedVersion"] == local.SOURCE for t in targets),
                "Expected two eligible connected Agent installations")
        print("Direct and Edge Agents connected; both have an update available", flush=True)
        return password, token, targets

    def run_agents(self):
        _, token, targets = self.setup_agents()
        for kind in ("direct", "edge"):
            selected = next(t for t in targets if t["name"] == f"fixture-{kind}-agent")
            review = self.http("updates/agents/review", {"targets": [selected["id"]]}, token)
            self.approval_attempted = True
            operation = self.http("updates/agents/approve",
                {"reviewId": review["id"], "requestId": str(uuid.uuid4())}, token)
            def outcome():
                result = self.http("updates/agents/operations/" + operation["id"], token=token)
                return result if result["finished"] else None
            result = self.wait_for(outcome, "Agent update did not finish", seconds=600)
            require(len(result["targets"]) == 1 and result["targets"][0]["phase"] == "Succeeded",
                    "Agent rollout failed; inspect the private fixture logs")
            if kind == "direct":
                edge = json.loads(self.nested("edge", "inspect", "citadel-agent").stdout)[0]
                require(edge["Id"] == self.agent_before["edge"]["Id"], "Unselected Agent was replaced")
                print("Direct updated; unselected Edge Agent is unchanged", flush=True)
        state = self.http("updates/agents", token=token)
        require(len(state["targets"]) == 2 and all(t["status"] == "Current" for t in state["targets"]),
                "Agents did not reconnect at the Core version")
        require({t["id"] for t in targets} == {t["id"] for t in state["targets"]},
                "Agent installation identities changed")
        core = self.json_command("inspect", self.core_before[0])[0]
        require(local.upgrade.database_state(core) == self.core_before, "Agent update restarted Core")
        for kind in ("direct", "edge"):
            before = self.agent_before[kind]
            after = json.loads(self.nested(kind, "inspect", "citadel-agent").stdout)[0]
            require(after["Id"] != before["Id"] and after["Config"]["Image"] == self.agent_target,
                    "Agent was not replaced by the expected immutable target")
            require(mount_identity(after) == mount_identity(before), "Agent identity mounts changed")
            require(published_ports(after) == published_ports(before),
                    "Agent published ports changed")
            if kind == "edge":
                old_networks = before["NetworkSettings"]["Networks"]
                new_networks = after["NetworkSettings"]["Networks"]
                require(set(old_networks) == set(new_networks), "Agent network attachments changed")
                for name, endpoint in old_networks.items():
                    require(endpoint["NetworkID"] == new_networks[name]["NetworkID"], "Agent network identity changed")
                    require(endpoint.get("GwPriority", 0) == new_networks[name].get("GwPriority", 0), "Agent gateway priority changed")
                    aliases = set(endpoint.get("Aliases") or []) - {before["Id"], before["Id"][:12]}
                    require(aliases <= set(new_networks[name].get("Aliases") or []), "Agent network aliases changed")
                helpers = self.nested(kind, "ps", "-aq", "--filter", "label=com.citadel.system-role=agent-updater").stdout.decode().split()
                require(len(helpers) == 1, "Expected one retained Edge update helper")
                helper = json.loads(self.nested(kind, "inspect", helpers[0]).stdout)[0]
                require(helper["Mounts"][0]["Source"] == self.socket(kind), "Helper selected the wrong Docker socket")
                require(helper["State"]["ExitCode"] == 0, "Edge helper did not finish successfully")
            retained = json.loads(self.nested(kind, "inspect", before["Id"]).stdout)[0]
            require(not retained["State"]["Running"], "Previous Agent is still running")
        print("Direct and Edge updates succeeded: authenticated reconnect, target digest, "
              "preserved mounts/ports and retained source containers verified", flush=True)

    def serve_agents(self, port):
        password, _, _ = self.setup_agents(browser_port=port)
        print(f"\nOpen http://localhost:{port}/updates\n"
              f"Email: upgrade@example.test\nPassword: {password}\n\n"
              "In Config → Available updates, select the Direct and Edge Agents.\n"
              "Click Review 2 Agent updates, then Update now.\n"
              f"Review and approve {local.SOURCE} → {local.TARGET}. Core stays running.\n"
              "Keep this process running. Ctrl+C removes only this disposable fixture.\n", flush=True)
        while True:
            time.sleep(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime-image", required=True, help="Local Core runtime base image")
    parser.add_argument("--agent-runtime-image", required=True, help="Local Agent runtime base image")
    parser.add_argument("--browser", action="store_true", help="Leave approval to the browser user")
    parser.add_argument("--port", type=int, default=18080)
    args = parser.parse_args()
    if not 1 <= args.port <= 65535:
        parser.error("--port must be between 1 and 65535")
    def interrupted(*_):
        raise KeyboardInterrupt()
    signal.signal(signal.SIGTERM, interrupted)
    images = []
    fixture = None
    with tempfile.TemporaryDirectory(prefix="citadel-local-agent-upgrade-") as temporary:
        directory = Path(temporary)
        try:
            frontend = None
            if args.browser:
                subprocess.run(["npm", "run", "build:image", "--prefix", "src/frontend"],
                    cwd=registration.ROOT, env=dict(os.environ, VITE_API_BASE_URL=""), check=True)
                frontend = registration.ROOT / "src/frontend/dist"
            catalog = hashlib.sha256((registration.ROOT /
                "src/infrastructure/database/generated/migrations.json").read_bytes()).hexdigest()
            images.append(local.build(args.runtime_image, local.TARGET, directory, catalog, frontend))
            for version in (local.SOURCE, local.TARGET):
                images.append(build_agent(args.agent_runtime_image, version, directory))
            fixture = AgentUpgrade(images[0], directory)
            fixture.start(registry=True, browser_port=args.port if args.browser else None)
            fixture.prepare_agents(images[1], images[2])
            if args.browser:
                fixture.serve_agents(args.port)
            else:
                fixture.run_agents()
        except BaseException as error:
            if args.browser and isinstance(error, KeyboardInterrupt):
                print("\nRemoving browser Agent fixture", flush=True)
            else:
                if fixture and fixture.daemon:
                    fixture.approval_attempted = True
                    fixture.capture_failure()
                raise
        finally:
            if fixture:
                fixture.close()
            for image in images:
                registration.run([*registration.DOCKER, "image", "rm", image], check=False)


if __name__ == "__main__":
    main()
