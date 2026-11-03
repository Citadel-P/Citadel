#!/usr/bin/env python3
"""Opt-in registration/revalidation acceptance on a disposable isolated Docker daemon.

Requires a Core image built from this checkout, Docker and Python 3 on the test
host. Python is test tooling only; registration itself runs from the Core image.
"""

import argparse
import json
from pathlib import Path
import secrets
import signal
import subprocess
import tempfile
import time
import uuid


ROOT = Path(__file__).resolve().parents[2]
DOCKER = ["docker", "--host", "unix:///var/run/docker.sock"]
# Same fixture versions as Agent acceptance; no daemon TCP port is published.
DIND = "docker@sha256:3f3c01aaaebf7cce837356b688b7c059a4749f10bd7660dec7c58fc454a283f0"
POSTGRES = "postgres@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5"


def run(args, *, timeout=120, check=True, stdin=None):
    try:
        result = subprocess.run(args, stdin=stdin, capture_output=True, timeout=timeout, check=False)
    except subprocess.TimeoutExpired:
        raise RuntimeError("Fixture command timed out; arguments and output withheld") from None
    if check and result.returncode:
        # Compose, inspect and HTTP output can include fixture credentials/tokens.
        raise RuntimeError(f"Fixture command failed (exit {result.returncode}); output withheld")
    return result


def require(condition, message):
    if not condition:
        raise AssertionError(message)


class Fixture:
    def __init__(self, core, directory):
        self.core = core
        self.directory = directory
        self.daemon = None
        self.network = None
        self.tag = f"ghcr.io/citadel-p/citadel:registration-test-{uuid.uuid4().hex}"

    def host(self, *args, **kwargs):
        return run([*DOCKER, *args], **kwargs)

    def execute(self, *args, **kwargs):
        return self.host("exec", self.daemon, *args, **kwargs)

    def docker(self, *args, **kwargs):
        return self.execute("docker", "--host", "unix:///var/run/docker.sock", *args, **kwargs)

    def start(self):
        print("Preparing disposable Docker daemon and loading test images", flush=True)
        # The caller supplies the exact Core image; never fetch an implicit latest.
        self.host("image", "inspect", self.core)
        if self.host("image", "inspect", DIND, check=False).returncode:
            self.host("pull", DIND, timeout=300)
        self.network = self.host("network", "create", f"citadel-update-acceptance-{uuid.uuid4().hex}").stdout.decode().strip()
        self.daemon = self.host(
            "create", "--privileged", "--network", self.network,
            "--name", f"citadel-update-acceptance-{uuid.uuid4().hex}",
            "-e", "DOCKER_TLS_CERTDIR=", DIND,
            "--host=unix:///var/run/docker.sock", "--tls=false",
        ).stdout.decode().strip()
        self.host("start", self.daemon)
        deadline = time.monotonic() + 90
        while self.docker("info", check=False).returncode:
            require(time.monotonic() < deadline, "Disposable Docker daemon did not become ready")
            time.sleep(1)
        # Docker save/load can discard RepoDigests. Pull the exact installer pin
        # inside the disposable daemon, then disconnect it before starting Core.
        self.docker("pull", POSTGRES, timeout=300)
        self.host("network", "disconnect", self.network, self.daemon)
        self.host("network", "rm", self.network)
        self.network = None
        archive = self.directory / "images.tar"
        self.host("image", "save", "--output", str(archive), self.core, timeout=300)
        # Stream into the daemon CLI: DinD's /tmp is private to its mount namespace.
        with archive.open("rb") as source:
            loaded = self.host("exec", "-i", self.daemon, "docker", "--host",
                               "unix:///var/run/docker.sock", "image", "load",
                               stdin=source, timeout=300, check=False)
        require(loaded.returncode == 0, "Fixture image import failed: " + loaded.stderr.decode(errors="replace"))
        archive.unlink()
        # This tag exists only inside the disposable daemon and is never pushed.
        self.docker("tag", self.core, self.tag)

    def close(self):
        try:
            if self.daemon:
                self.host("rm", "-fv", self.daemon, timeout=120)
        finally:
            if self.network:
                self.host("network", "rm", self.network)

    def write(self, path, data, mode="600"):
        local = self.directory / "input"
        local.write_bytes(data if isinstance(data, bytes) else data.encode())
        local.chmod(0o600)
        self.host("cp", str(local), f"{self.daemon}:{path}")
        self.execute("chown", "0:0", path)
        self.execute("chmod", mode, path)

    def read(self, path):
        return self.execute("cat", path).stdout

    def profile(self, edge):
        project = "citadel_edge" if edge else "citadel_base"
        directory = f"/opt/{project}"
        state = f"/opt/{project}-updates"
        self.execute("mkdir", "-p", directory, state)
        self.execute("chmod", "700", directory, state)
        sources = ["docker-compose.yml"] + (["compose.edge.yml"] if edge else [])
        files = ["installation.yaml", "edge-ports.yaml"] if edge else sources
        for source, name in zip(sources, files):
            content = (ROOT / "deploy/install" / source).read_text()
            if edge:
                content = "# Local installation notes; not a byte-for-byte template.\n" + content
                if source == "docker-compose.yml":
                    for role, actual in [("citadel_data", "acceptance_edge_core"),
                                         ("postgres_data", "acceptance_edge_database")]:
                        marker = f"  {role}:\n"
                        require(content.count(marker) == 1, "Fixture volume role changed")
                        content = content.replace(marker, marker + f"    name: {actual}\n")
                    content += "\nnetworks:\n  default:\n    name: acceptance_edge_network\n"
            self.write(f"{directory}/{name}", content)
        password = secrets.token_hex(24)
        env = (
            f"CITADEL_IMAGE={self.tag}\nPG_PASSWORD={password}\n"
            f"CITADEL_HTTP_PORT={18010 if edge else 18000}\n"
            "CITADEL_UPDATE_CHECKS_ENABLED=false\n"
        )
        self.write(f"{directory}/.env", env)
        compose = ["compose", "--project-name", project, "--project-directory", directory,
                   "--env-file", f"{directory}/.env"]
        for name in files:
            compose += ["--file", f"{directory}/{name}"]
        print(f"{project}: starting registered Compose installation", flush=True)
        started = self.docker(*compose, "up", "--detach", "--pull", "never", "--wait", "--wait-timeout", "180",
                              timeout=210, check=False)
        require(started.returncode == 0,
                "Installer startup failed: " + started.stderr.decode(errors="replace").replace(password, "[redacted]"))
        core = self.docker(*compose, "ps", "--quiet", "server").stdout.decode().strip()
        database = self.docker(*compose, "ps", "--quiet", "pg_db").stdout.decode().strip()
        before = json.loads(self.docker("inspect", core, database).stdout)

        command = ["run", "--rm", "--pull", "never", "--network", "none", "--user", "0:0",
                   "--entrypoint", "/app/citadel-server",
                   "--mount", f"type=bind,src={directory},dst={directory},readonly",
                   "--mount", f"type=bind,src={state},dst={state}",
                   "--mount", "type=bind,src=/var/run/docker.sock,dst=/var/run/docker.sock",
                   self.tag, "register-update-installation", "--directory", directory,
                   "--project", project, "--state-directory", state]
        for name in files:
            command += ["--compose-file", f"{directory}/{name}"]

        check_command = command[:command.index("register-update-installation")] + [
            "check-update-installation", "--state-directory", state,
        ]

        def invoke(args, success, expected_error=None):
            result = self.docker(*args, check=False, timeout=180)
            require(password.encode() not in result.stdout + result.stderr, "Installation diagnostic leaked a secret")
            require((result.returncode == 0) == success,
                    "Unexpected installation diagnostic outcome: " + result.stderr.decode(errors="replace"))
            if expected_error:
                require(expected_error.encode() in result.stderr, "Installation diagnostic failed for an unexpected reason")
            return json.loads(result.stdout) if success else None

        def register(success, expected_error=None):
            return invoke(command, success, expected_error)

        def check(success, expected_error=None):
            return invoke(check_command, success, expected_error)

        check(False)
        require(self.execute("test", "-e", f"{state}/installation.lock", check=False).returncode != 0,
                "Live check created a missing lock file")
        register(False, "initialized, reachable Citadel database")
        require(self.execute("test", "-e", f"{state}/installation.json", check=False).returncode != 0,
                "Registration must not persist before instance setup")
        port = "18010" if edge else "18000"
        # HTTP setup is the production path; no fixture SQL or fabricated identity.
        self.execute("wget", "-qO-", "--header=Content-Type: application/json",
                     "--post-data=" + json.dumps({"name": "acceptance", "email": "acceptance@example.test",
                                                 "password": secrets.token_hex(24)}),
                     f"http://127.0.0.1:{port}/api/v1/setup/initialize")
        first = register(True)
        require(first["kind"] == "DockerCompose", "Registration lost its installation kind")
        if edge:
            require((first["coreVolume"], first["databaseVolume"], first["network"])
                    == ("acceptance_edge_core", "acceptance_edge_database", "acceptance_edge_network"),
                    "Registration ignored custom resource names")
        original = self.read(f"{state}/installation.json")
        checked = check(True)
        require(checked["database"]["instanceId"] == first["instanceId"], "Database preflight bound the wrong instance")
        require(len(checked["database"]["catalogSha256"]) == 64, "Database preflight omitted catalog evidence")
        require(checked["recoveryKeys"] == {"jwt": "DataVolume", "secretsEncryption": "DataVolume", "agentSigning": "DataVolume"},
                "Recovery keys were not verified in the Core data volume")
        require(checked["checkpointStorage"]["availableBytes"] >= checked["checkpointStorage"]["minimumBytes"],
                "Checkpoint storage did not meet its admission budget")
        identity = json.loads(self.docker("exec", core, "/app/citadel-server", "instance-identity-json").stdout)
        require(first["instanceId"] == identity["instanceId"], "Registration bound the wrong instance")
        require(first["coreContainerId"] == core and first["databaseContainerId"] == database,
                "Registration bound the wrong containers")
        require(register(True) == first and self.read(f"{state}/installation.json") == original,
                "Idempotent retry changed registration")
        require(check(True)["registration"] == first and self.read(f"{state}/installation.json") == original,
                "Live check changed registration")
        mode = self.execute("stat", "-c", "%a:%u:%g", f"{state}/installation.json").stdout.strip()
        require(mode == b"600:0:0", "Registration permissions or ownership are unsafe")
        require(password.encode() not in original, "Registration stored a secret")

        self.write(f"{directory}/.env", env + "CITADEL_HTTP_ALLOWED_HOSTS=changed.example.test\n")
        check(False, "Registered installer inputs have changed")
        register(False, "Running service environment differs")
        require(self.read(f"{state}/installation.json") == original, "Input drift overwrote registration")
        self.write(f"{directory}/.env", env)
        self.execute("chmod", "644", f"{directory}/.env")
        check(False, "installer .env must be private")
        register(False, "installer .env must be private")
        self.execute("chmod", "600", f"{directory}/.env")

        # A stopped consumer still owns a volume; it must prevent registration.
        extra = self.docker("create", "--network", "none", "--entrypoint", "/bin/true",
                            "--mount", f"type=volume,src={first['coreVolume']},dst=/shared", self.tag).stdout.decode().strip()
        register(False, "persistent volume is shared")
        check(False, "persistent volume is shared")
        self.docker("rm", extra)
        extra = self.docker("run", "--detach", "--network", first["network"],
                            "--entrypoint", "/bin/sleep", self.tag, "300").stdout.decode().strip()
        register(False, "database network identity is ambiguous")
        check(False, "database network identity is ambiguous")
        self.docker("rm", "--force", extra)
        require(self.read(f"{state}/installation.json") == original, "Rejected drift changed evidence")

        conflicting = dict(first, instanceId=str(uuid.uuid4()))
        evidence = json.dumps(conflicting).encode()
        self.write(f"{state}/installation.json", evidence)
        check(False, "live installation no longer matches")
        register(False, "different installation is already registered")
        require(self.read(f"{state}/installation.json") == evidence, "Conflicting evidence was overwritten")
        self.write(f"{state}/installation.json", original)
        self.write(f"{state}/recovery.json", b"preserve recovery evidence")
        check(False, "execution evidence requires recovery")
        require(self.read(f"{state}/recovery.json") == b"preserve recovery evidence",
                "Live check changed recovery evidence")
        self.execute("rm", f"{state}/recovery.json")
        require(check(True)["registration"] == first, "Live check failed after fixture drift was removed")
        require(register(True) == first, "Registration did not recover after fixture drift was removed")
        after = json.loads(self.docker("inspect", core, database).stdout)
        require([(v["Id"], v["State"]["StartedAt"], v["RestartCount"]) for v in before]
                == [(v["Id"], v["State"]["StartedAt"], v["RestartCount"]) for v in after],
                "Registration restarted or replaced a service")
        print(f"{project}: registration, live revalidation, drift and evidence preservation passed", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("core_image", help="Core image built from the current checkout")
    args = parser.parse_args()

    def interrupted(*_):
        raise KeyboardInterrupt()

    signal.signal(signal.SIGTERM, interrupted)
    with tempfile.TemporaryDirectory(prefix="citadel-update-acceptance-") as temporary:
        fixture = Fixture(args.core_image, Path(temporary))
        try:
            fixture.start()
            fixture.profile(edge=False)
            fixture.profile(edge=True)
        finally:
            fixture.close()


if __name__ == "__main__":
    main()
