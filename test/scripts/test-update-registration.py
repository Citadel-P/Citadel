#!/usr/bin/env python3
"""Packaged registration/revalidation acceptance on a disposable isolated Docker daemon.

Requires a Core image built from this checkout, Docker and Python 3 on the test
host. Rust registration runs from the Core image. Optional activation acceptance
runs the shipped Python host setup command inside the disposable daemon.
"""

import argparse
import json
from pathlib import Path
import secrets
import re
import signal
import subprocess
import tempfile
import time
import uuid


ROOT = Path(__file__).resolve().parents[2]
DOCKER = ["docker", "--host", "unix:///var/run/docker.sock"]
# Same fixture versions as Agent acceptance; no daemon TCP port is published.
DIND = "docker@sha256:3f3c01aaaebf7cce837356b688b7c059a4749f10bd7660dec7c58fc454a283f0"
REGISTRY = "registry@sha256:d4032d4cc323c1731c707931ecada07a53076eac38e3b61ae57533f170f2c6c3"
POSTGRES = "postgres@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5"


def run(args, *, timeout=120, check=True, stdin=None):
    try:
        result = subprocess.run(args, stdin=stdin, capture_output=True, timeout=timeout, check=False)
    except subprocess.TimeoutExpired:
        raise RuntimeError("Fixture command timed out; arguments and output withheld") from None
    if check and result.returncode:
        # Compose, inspect and HTTP output can include fixture credentials/tokens.
        evidence = Path(tempfile.mkdtemp(prefix="citadel-fixture-command-failure-")) / "output.log"
        evidence.write_bytes(result.stdout + b"\n" + result.stderr)
        evidence.chmod(0o600)
        raise RuntimeError(f"Fixture command failed (exit {result.returncode}); private output: {evidence}")
    return result


def require(condition, message):
    if not condition:
        raise AssertionError(message)


class Fixture:
    def __init__(self, core, directory, *, rootless=False):
        self.rootless = rootless
        self.socket = "/run/user/1000/docker.sock" if rootless else "/var/run/docker.sock"
        self.core = core
        self.directory = directory
        self.daemon = None
        self.network = None
        self.tag = f"ghcr.io/citadel-p/citadel:registration-test-{uuid.uuid4().hex}"

    def host(self, *args, **kwargs):
        return run([*DOCKER, *args], **kwargs)

    def execute(self, *args, **kwargs):
        interactive = ["--interactive"] if kwargs.get("stdin") is not None else []
        return self.host("exec", *interactive, *(["--user=0:0"] if self.rootless else []), self.daemon, *args, **kwargs)

    def activate(self, *args, **kwargs):
        return self.host('exec', *(['--user=1000:1000'] if self.rootless else []),
                         '-e', 'DOCKER_HOST=unix://' + self.socket, self.daemon, *args, **kwargs)

    def docker(self, *args, **kwargs):
        return self.execute("docker", "--host", "unix://" + self.socket, *args, **kwargs)

    def start(self, *, published=False, registry=False, browser_port=None, activation=False, automatic=False):
        registry = registry or activation or automatic
        require(not (published and registry), "Fixture registry cannot be used for published releases")
        require(browser_port is None or (registry and 1 <= browser_port <= 65535),
                "Browser access requires the private registry and a valid local port")
        print("Preparing disposable Docker daemon and loading test images", flush=True)
        # The caller supplies the exact Core image; never fetch an implicit latest.
        if published:
            require(re.fullmatch(r"ghcr\.io/citadel-p/citadel@sha256:[0-9a-f]{64}", self.core),
                    "Published acceptance requires an exact official Core digest")
        else:
            self.host("image", "inspect", self.core)
        dind = "docker@sha256:e3d0fa70055956693134873845b743bbe35b5199c68a52347032ae3e5dbac9a0" if self.rootless else DIND
        if self.host("image", "inspect", dind, check=False).returncode:
            self.host("pull", dind, timeout=300)
        self.network = self.host("network", "create", f"citadel-update-acceptance-{uuid.uuid4().hex}").stdout.decode().strip()
        self.daemon = self.host(
            "create", "--privileged", "--network", self.network,
            "--name", f"citadel-update-acceptance-{uuid.uuid4().hex}",
            *(["--publish", f"127.0.0.1:{browser_port}:18000"] if browser_port else []),
            *(["--add-host", "ghcr.io:127.0.0.1"] if registry else []),
            "-e", "DOCKER_TLS_CERTDIR=", dind,
            *(["--insecure-registry=ghcr.io"] if registry else []),
            "--host=unix://" + self.socket, "--tls=false",
        ).stdout.decode().strip()
        self.host("start", self.daemon)
        deadline = time.monotonic() + 90
        while self.docker("info", check=False).returncode:
            require(time.monotonic() < deadline, "Disposable Docker daemon did not become ready")
            time.sleep(1)
        info = json.loads(self.docker("info", "--format", "{{json .}}").stdout)
        require(("name=rootless" in info["SecurityOptions"]) == self.rootless, "Fixture daemon mode is incorrect")
        self.docker_root = info["DockerRootDir"]
        # Docker save/load can discard RepoDigests. Pull the exact installer pin
        # inside the disposable daemon, then disconnect it before starting Core.
        self.docker("pull", POSTGRES, timeout=300)
        if registry:
            self.docker("pull", REGISTRY, timeout=300)
        if published:
            # Retain registry/GitHub access and RepoDigests for the real verifier.
            # No host ports or host Docker socket are exposed to this daemon.
            self.docker("pull", self.core, timeout=300)
            self.tag = self.core
            return
        if activation:
            self.execute("apk", "add", "--no-cache", "python3", timeout=180)
        if not browser_port:
            self.host("network", "disconnect", self.network, self.daemon)
            self.host("network", "rm", self.network)
            self.network = None
        archive = self.directory / "images.tar"
        self.host("image", "save", "--output", str(archive), self.core, timeout=300)
        # Stream into the daemon CLI: DinD's /tmp is private to its mount namespace.
        with archive.open("rb") as source:
            loaded = self.host("exec", "-i", self.daemon, "docker", "--host",
                               "unix://" + self.socket, "image", "load",
                               stdin=source, timeout=300, check=False)
        require(loaded.returncode == 0, "Fixture image import failed: " + loaded.stderr.decode(errors="replace"))
        archive.unlink()
        # This tag exists only inside the disposable daemon and is never pushed.
        self.docker("tag", self.core, self.tag)
        if activation or automatic:
            # Startup verifies the installed image's RepoDigest. Give this local
            # candidate a real digest through an isolated loopback registry;
            # ghcr.io resolves ONLY to this disposable daemon, never public GHCR.
            registry_id = self.docker('run', '-d', '--network=host', '-e', 'REGISTRY_HTTP_ADDR=0.0.0.0:80', REGISTRY).stdout.decode().strip()
            deadline = time.monotonic() + 30
            while self.docker('exec', registry_id, 'wget', '-qO-', 'http://127.0.0.1/v2/', check=False).returncode:
                require(time.monotonic() < deadline, 'Private fixture registry did not become ready')
                time.sleep(1)
            self.docker('push', self.tag, timeout=300)


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
        self.execute("chown", "1000:1000" if self.rootless else "0:0", path)
        self.execute("chmod", mode, path)

    def read(self, path):
        return self.execute("cat", path).stdout

    def profile(self, edge, *, activation=False, automatic=False):
        project = "citadel_edge" if edge else "citadel_base"
        directory = f"/opt/{project}"
        state = f"{directory}/.citadel-updates" if automatic else f"/opt/{project}-updates"
        directories = [directory] if automatic else [directory, state]
        self.execute("mkdir", "-p", *directories)
        self.execute("chmod", "700", *directories)
        if self.rootless:
            self.execute("chown", "1000:1000", *directories)
        sources = ["docker-compose.yml"] + (["compose.edge.yml"] if edge else [])
        files = ["installation.yaml", "edge-ports.yaml"] if edge else sources
        for source, name in zip(sources, files):
            content = (ROOT / "deploy/install" / source).read_text()
            if not automatic:
                # Explicit custom deployment exercises the manual registration path.
                content = content.replace('      CITADEL_UPDATE_STATE_DIRECTORY: /var/lib/citadel-updates\n', '')
                content = content.replace('      - ./.citadel-updates:/var/lib/citadel-updates:ro\n', '')
            if self.rootless and source == "docker-compose.yml":
                content = content.replace('/var/run/docker.sock:/var/run/docker.sock', f'{self.socket}:/var/run/docker.sock')
                content = content.replace('- /:/host:ro', f'- {self.docker_root}:/host{self.docker_root}:ro')
            if edge:
                content = "# Local installation notes; not a byte-for-byte template.\n" + content
                if source == "docker-compose.yml":
                    for role, actual in [("citadel_data", "acceptance_edge_core"),
                                         ("postgres_data", "acceptance_edge_database")]:
                        marker = f"  {role}:\n"
                        require(content.count(marker) == 1, "Fixture volume role changed")
                        content = content.replace(marker, marker + f"    name: {actual}\n")
                    content = content.replace("  server:\n", "  server:\n    cpus: 0.75\n    mem_limit: 0\n    memswap_limit: 0\n", 1)
                    content += "\nnetworks:\n  default:\n    name: acceptance_edge_network\n    ipam:\n      config:\n        - subnet: 172.30.231.0/24\n          gateway: 172.30.231.1\n"
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
                   "--mount", f"type=bind,src={self.socket},dst=/var/run/docker.sock",
                   self.tag, "register-update-installation", "--directory", directory,
                   "--project", project, "--state-directory", state]
        for name in files:
            command += ["--compose-file", f"{directory}/{name}"]

        check_command = command[:command.index("register-update-installation")] + [
            "check-update-installation", "--state-directory", state,
        ]

        def invoke(args, success, expected_error=None, stdin=None):
            result = self.docker(*args, check=False, timeout=180, stdin=stdin)
            require(password.encode() not in result.stdout + result.stderr, "Installation diagnostic leaked a secret")
            require((result.returncode == 0) == success,
                    "Unexpected installation diagnostic outcome: " + result.stderr.decode(errors="replace"))
            if expected_error:
                require(expected_error.encode() in result.stderr, "Installation diagnostic failed for an unexpected reason: " + result.stderr.decode(errors="replace"))
            return json.loads(result.stdout) if success else None

        def register(success, expected_error=None):
            return invoke(command, success, expected_error)

        def check(success, expected_error=None):
            return invoke(check_command, success, expected_error)

        def refresh(success, expected_error=None):
            return invoke([*command, "--refresh"], success, expected_error)

        def host_check(action, success=True):
            # Same executable, mounts and restricted runtime as HostUpdateClient.
            # This exercises the packaged utility, not release signature admission.
            utility = command[:command.index(self.tag)] + [
                "--interactive", "--read-only", "--restart=no", "--no-healthcheck",
                "--env", "TMPDIR=/tmp", "--tmpfs=/tmp:rw,noexec,nosuid,size=64m",
                self.tag, "update-host",
            ]
            with tempfile.TemporaryFile() as request:
                request.write(json.dumps({"action": action, "request": {"state_directory": state}}).encode())
                request.seek(0)
                return invoke(utility, success, stdin=request)

        if not automatic:
            check(False)
            require(self.execute("test", "-e", f"{state}/installation.lock", check=False).returncode != 0,
                    "Live check created a missing lock file")
            refresh(False)
            register(False, "initialized, reachable Citadel database")
            require(self.execute("test", "-e", f"{state}/installation.json", check=False).returncode != 0,
                    "Registration must not persist before instance setup")
        port = "18010" if edge else "18000"
        # HTTP setup is the production path; no fixture SQL or fabricated identity.
        setup_response = self.execute("wget", "-qO-", "--header=Content-Type: application/json",
                     "--post-data=" + json.dumps({"name": "acceptance", "email": "acceptance@example.test",
                                                 "password": secrets.token_hex(24)}),
                     f"http://127.0.0.1:{port}/api/v1/setup/initialize")
        if automatic:
            self.check_automatic(directory, state, compose, core, database, before, check, port, json.loads(setup_response.stdout)["accessToken"])
            return
        if activation:
            self.activate_updates(directory, state, project, files, compose, core, before[1], check)
            return
        first = register(True)
        require(first["kind"] == "DockerCompose", "Registration lost its installation kind")
        if edge:
            require((first["coreVolume"], first["databaseVolume"], first["network"])
                    == ("acceptance_edge_core", "acceptance_edge_database", "acceptance_edge_network"),
                    "Registration ignored custom resource names")
        original = self.read(f"{state}/installation.json")
        require(host_check("startup") == {"status": "registered", "result": first},
                "Host utility did not return the registered instance")
        require(host_check("check")["result"]["registration"] == first,
                "Read-only host utility could not revalidate the packaged installation")
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

        print(f"{project}: live preflight and host utility passed; checking drift rejection", flush=True)
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
        refresh(False)
        host_check("startup", False)
        require(self.read(f"{state}/recovery.json") == b"preserve recovery evidence",
                "Live check changed recovery evidence")
        self.execute("rm", f"{state}/recovery.json")
        require(check(True)["registration"] == first, "Live check failed after fixture drift was removed")
        require(register(True) == first, "Registration did not recover after fixture drift was removed")
        after = json.loads(self.docker("inspect", core, database).stdout)
        require([(v["Id"], v["State"]["StartedAt"], v["RestartCount"]) for v in before]
                == [(v["Id"], v["State"]["StartedAt"], v["RestartCount"]) for v in after],
                "Registration restarted or replaced a service")
        print(f"{project}: drift rejected; recreating Core to verify explicit refresh", flush=True)
        # An intentional Core-only configuration change requires explicit refresh.
        # Local candidate archives have no registry RepoDigest, so do not enable
        # release installation or pretend this qualifies a signed transition.
        keys = self.docker("exec", core, "sha256sum", "/app/data/jwtsecret",
                           "/app/data/secret-encryption-key", "/app/data/agent/signing-key").stdout
        require(keys.strip(), "Fixture did not retain file-backed recovery keys")
        self.write(f"{directory}/.env", env + "CITADEL_HTTP_ALLOWED_HOSTS=localhost;127.0.0.1;acceptance.test\n")
        self.docker(*compose, "up", "--detach", "--no-deps", "--pull", "never",
                    "--wait", "--wait-timeout", "180", "server", timeout=210)
        replacement = self.docker(*compose, "ps", "--quiet", "server").stdout.decode().strip()
        require(replacement != core, "Fixture did not recreate Core for the configuration change")
        register(False, "different installation is already registered")
        check(False)
        require(self.read(f"{state}/installation.json") == original, "Implicit refresh changed registration")
        refreshed = refresh(True)
        expected = dict(first, coreContainerId=replacement,
                        configurationSha256=refreshed["configurationSha256"],
                        runtimeSha256=refreshed["runtimeSha256"])
        require(refreshed == expected, "Refresh changed installation or storage identity")
        require(refreshed["configurationSha256"] != first["configurationSha256"],
                "Refresh did not bind the changed configuration")
        require(refresh(True) == refreshed and check(True)["registration"] == refreshed,
                "Refreshed installation did not pass an idempotent retry and live check")
        require(host_check("check")["result"]["registration"] == refreshed,
                "Host utility did not observe refreshed registration")
        after_database = json.loads(self.docker("inspect", database).stdout)[0]
        require((after_database["Id"], after_database["State"]["StartedAt"], after_database["RestartCount"])
                == (before[1]["Id"], before[1]["State"]["StartedAt"], before[1]["RestartCount"]),
                "Registration refresh restarted or replaced PostgreSQL")
        require(self.docker("exec", replacement, "sha256sum", "/app/data/jwtsecret",
                            "/app/data/secret-encryption-key", "/app/data/agent/signing-key").stdout == keys,
                "Core recreation changed recovery keys")
        print(f"{project}: registration, host utility, refresh and evidence preservation passed", flush=True)

    def check_automatic(self, directory, state, compose, core, database, before, check, port, token):
        print('Verifying automatic Compose registration without a host installer', flush=True)
        original_env = self.read(f'{directory}/.env')
        deadline = time.monotonic() + 210
        while True:
            registration = json.loads(self.read(f'{state}/installation.json'))
            if registration.get('kind') == 'DockerCompose':
                break
            if time.monotonic() >= deadline:
                evidence = Path(tempfile.mkdtemp(prefix='citadel-auto-setup-failure-')) / 'core.log'
                evidence.write_bytes(self.docker('logs', core).stdout + self.docker('logs', core).stderr)
                evidence.chmod(0o600)
                raise AssertionError(f'Automatic registration did not complete; private logs: {evidence}')
            time.sleep(2)
        require(registration['stateDirectory'] == state, 'Registration did not resolve the host state path')
        require(registration['coreContainerId'] == core and registration['databaseContainerId'] == database,
                'Automatic registration changed service identities')
        check(True)
        deadline = time.monotonic() + 30
        while True:
            status = json.loads(self.execute('wget', '-qO-', '--header=Authorization: Bearer ' + token,
                                            f'http://127.0.0.1:{port}/api/v1/updates').stdout)
            if status.get('databaseManaged'):
                break
            require(time.monotonic() < deadline, 'Updates status did not observe automatic registration')
            time.sleep(1)
        after = json.loads(self.docker('inspect', core, database).stdout)
        for original, current in zip(before, after):
            require(current['State']['StartedAt'] == original['State']['StartedAt']
                    and current['RestartCount'] == original['RestartCount'], 'Registration restarted a service')
        require(self.read(f'{directory}/.env') == original_env, 'Registration edited .env')
        saved = self.read(f'{state}/installation.json')
        # A normal Core restart must admit completed registration without changing it.
        self.docker(*compose, 'restart', 'server', timeout=90)
        deadline = time.monotonic() + 90
        while True:
            inspected = json.loads(self.docker('inspect', core).stdout)[0]
            if inspected['State'].get('Health', {}).get('Status') == 'healthy':
                break
            require(time.monotonic() < deadline, 'Registered Core did not become healthy after restart')
            time.sleep(2)
        require(self.read(f'{state}/installation.json') == saved, 'Restart rewrote registration')
        check(True)
        require(not self.docker('ps', '-aq', '--filter', 'label=com.citadel.update-host').stdout.strip(),
                'Automatic registration left helper containers')
        # Destructive negative case is confined to this disposable installation.
        self.docker(*compose, 'stop', 'server', timeout=90)
        self.execute('rm', f'{state}/installation.json')
        self.docker(*compose, 'start', 'server')
        deadline = time.monotonic() + 90
        while True:
            logs = self.docker('logs', '--tail', '50', core)
            if b'Core is in update maintenance' in logs.stdout + logs.stderr:
                break
            require(time.monotonic() < deadline, 'Missing registration did not enter maintenance')
            time.sleep(2)
        require(self.execute('test', '-e', f'{state}/installation.json', check=False).returncode != 0,
                'Missing completed registration was silently recreated')
        print('Automatic setup, unchanged services/configuration, restart and lost-state fencing verified', flush=True)

    def activate_updates(self, directory, state, project, files, compose, core, database, check):
        """Run the shipped one-time setup rather than manually reproducing its steps."""
        self.write("/opt/enable-updates.py", (ROOT / "deploy/install/enable-updates.py").read_bytes())
        keys = self.docker("exec", core, "sha256sum", "/app/data/jwtsecret",
                           "/app/data/secret-encryption-key", "/app/data/agent/signing-key").stdout
        image = json.loads(self.docker("inspect", core).stdout)[0]["Image"]
        args = ["python3", "/opt/enable-updates.py", "--start", "--directory", directory,
                "--project", project, "--state-directory", state]
        for name in files:
            args += ["--compose-file", f"{directory}/{name}"]
        self.activate(*args, timeout=600)
        replacement = self.docker(*compose, "ps", "--quiet", "server").stdout.decode().strip()
        require(replacement != core, "Activation did not recreate Core")
        current = json.loads(self.docker("inspect", replacement, database["Id"]).stdout)
        require(current[0]["Image"] == image, "Activation changed the Core release")
        require((current[1]["Id"], current[1]["State"]["StartedAt"], current[1]["RestartCount"])
                == (database["Id"], database["State"]["StartedAt"], database["RestartCount"]),
                "Activation changed PostgreSQL")
        require(self.docker("exec", replacement, "sha256sum", "/app/data/jwtsecret",
                            "/app/data/secret-encryption-key", "/app/data/agent/signing-key").stdout == keys,
                "Activation changed recovery keys")
        registered = self.read(f"{state}/installation.json")
        require(check(True)["registration"]["coreContainerId"] == replacement,
                "Activated registration did not match Core")
        self.activate(*args, timeout=300)
        require(self.read(f"{state}/installation.json") == registered,
                "Repeated activation changed registration")
        require(self.docker(*compose, "ps", "--quiet", "server").stdout.decode().strip() == replacement,
                "Repeated activation restarted Core")
        deployment_check = """
import argparse, runpy, subprocess, sys
from pathlib import Path
setup = runpy.run_path('/opt/enable-updates.py')
directory, project, state, *files = sys.argv[1:]
args = argparse.Namespace(directory=directory, project=project, state_directory=state, compose_file=files)
def replace():
    env = Path(directory) / '.env'
    env.write_text(env.read_text() + 'CITADEL_FIXTURE_DEPLOYMENT=1\\n')
    command = ['docker', 'compose', '--project-name', project, '--project-directory', directory, '--env-file', str(env)]
    for file in files:
        command += ['-f', file]
    subprocess.run([*command, 'up', '-d', '--no-deps', '--pull', 'never', '--wait', '--wait-timeout', '180', 'server'], check=True)
setup['enable'](args, replace=replace)
"""
        self.activate('python3', '-c', deployment_check, directory, project, state,
                     *[f'{directory}/{name}' for name in files], timeout=600)
        deployed = self.docker(*compose, 'ps', '--quiet', 'server').stdout.decode().strip()
        require(deployed != replacement, 'Registered deployment did not recreate Core')
        require(check(True)['registration']['coreContainerId'] == deployed,
                'Registered deployment did not refresh Core identity')
        after_database = json.loads(self.docker('inspect', database['Id']).stdout)[0]
        require((after_database['Id'], after_database['State']['StartedAt'], after_database['RestartCount'])
                == (database['Id'], database['State']['StartedAt'], database['RestartCount']),
                'Registered deployment changed PostgreSQL')
        print(f"{project}: automatic setup, idempotence and guarded deployment refresh passed", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("core_image", help="Core image built from the current checkout")
    parser.add_argument("--enable-updates", action="store_true", help="Exercise the shipped host activation command")
    parser.add_argument("--automatic", action="store_true", help="Exercise Compose-only automatic registration")
    parser.add_argument("--rootless", action="store_true", help="Use a disposable rootless Docker daemon")
    args = parser.parse_args()

    def interrupted(*_):
        raise KeyboardInterrupt()

    signal.signal(signal.SIGTERM, interrupted)
    with tempfile.TemporaryDirectory(prefix="citadel-update-acceptance-") as temporary:
        fixture = Fixture(args.core_image, Path(temporary), rootless=args.rootless)
        try:
            fixture.start(activation=args.enable_updates, automatic=args.automatic)
            fixture.profile(edge=False, activation=args.enable_updates, automatic=args.automatic)
            fixture.profile(edge=True, activation=args.enable_updates, automatic=args.automatic)
        finally:
            fixture.close()


if __name__ == "__main__":
    main()
