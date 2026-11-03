#!/usr/bin/env python3
"""Exercise an eligible published Core transition on a disposable Docker daemon.

Uses real signatures, review/approval, replacement and checkpoint restore. It does
not authorize a transition or supply a bypass for an empty embedded source list.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import secrets
import signal
import tempfile
import time
import uuid

spec = importlib.util.spec_from_file_location(
    "registration", Path(__file__).with_name("test-update-registration.py"))
registration = importlib.util.module_from_spec(spec)
spec.loader.exec_module(registration)
require = registration.require


class UpgradeFixture(registration.Fixture):
    installation = "/opt/citadel-upgrade"
    state = "/opt/citadel-upgrade-state"
    project = "citadel_upgrade"

    def json_command(self, *args):
        return json.loads(self.docker(*args, timeout=600).stdout)

    def http(self, path, data=None, token=None, *, optional=False):
        args = ["wget", "-qO-", "--timeout=30", "--tries=1", "--header=Content-Type: application/json"]
        if token:
            args += ["--header=Authorization: Bearer " + token]
        if data is not None:
            args += ["--post-data=" + json.dumps(data)]
        result = self.execute(*args, "http://127.0.0.1:18000/api/v1/" + path,
                              timeout=45, check=False)
        if optional and result.returncode:
            return None
        require(result.returncode == 0, "Fixture HTTP request failed; response withheld")
        return json.loads(result.stdout)

    def login(self, password, *, optional=False):
        result = self.http("authentication/login", {
            "emailOrName": "upgrade@example.test", "password": password,
        }, optional=optional)
        if result is None:
            return None
        require(result.get("nextStep") == "Completed" and result.get("accessToken"),
                "Fixture login did not complete")
        return result["accessToken"]

    def compose(self, *args, **kwargs):
        return self.docker("compose", "--project-name", self.project,
                           "--project-directory", self.installation,
                           "--env-file", self.installation + "/.env",
                           "--file", self.installation + "/docker-compose.yml", *args, **kwargs)

    def utility(self, command, *args):
        return self.json_command(
            "run", "--rm", "--pull=never", "--network=none", "--user=0:0",
            "--entrypoint=/app/citadel-server",
            "--mount", f"type=bind,src={self.installation},dst={self.installation},readonly",
            "--mount", f"type=bind,src={self.state},dst={self.state}",
            "--mount", "type=bind,src=/var/run/docker.sock,dst=/var/run/docker.sock",
            self.core, command, "--state-directory", self.state, *args)

    def register(self, refresh=False):
        return self.utility("register-update-installation", "--directory", self.installation,
                            "--project", self.project, "--compose-file",
                            self.installation + "/docker-compose.yml", *(["--refresh"] if refresh else []))

    def sql(self, database, statement):
        # Only the fixture-created database container is used; no host connection.
        return self.docker("exec", database, "psql", "-U", "citadel", "-d", "citadel",
                           "-XAt", "--set=ON_ERROR_STOP=1", "-c", statement).stdout.strip()

    def keys(self, core):
        return self.docker("exec", core, "sha256sum", "/app/data/jwtsecret",
                           "/app/data/secret-encryption-key", "/app/data/agent/signing-key").stdout

    def run_transition(self, target_version):
        architecture = self.json_command("image", "inspect", self.core)[0]["Architecture"]
        source = self.json_command("run", "--rm", "--network=none", self.core, "upgrade-descriptor-json")
        verified = self.json_command("run", "--rm", self.core, "verify-update-release",
                                     "--version", source["targetVersion"])
        require(verified["image"] == self.core, "Source digest differs from its signed release")
        target = self.json_command(
            "run", "--rm", "--mount", "type=bind,src=/var/run/docker.sock,dst=/var/run/docker.sock",
            self.core, "inspect-update-release", "--version", target_version,
            "--architecture", architecture)
        require_eligible(source, target, architecture)
        print("Signed source and target inspected; creating isolated installation", flush=True)
        self.execute("mkdir", "-p", self.installation, self.state)
        self.execute("chmod", "700", self.installation, self.state)
        self.write(self.installation + "/docker-compose.yml",
                   (registration.ROOT / "deploy/install/docker-compose.yml").read_text())
        password, admin_password = secrets.token_hex(24), secrets.token_hex(24)
        env = f"CITADEL_IMAGE={self.core}\nPG_PASSWORD={password}\n"
        self.write(self.installation + "/.env", env)
        self.compose("up", "-d", "--pull=never", "--wait", "--wait-timeout=180", timeout=210)
        self.http("setup/initialize", {"name": "upgrade", "email": "upgrade@example.test",
                                       "password": admin_password})
        self.register()
        self.write(self.installation + "/.env", env + f"CITADEL_UPDATE_STATE_DIRECTORY={self.state}\n")
        self.compose("up", "-d", "--no-deps", "--pull=never", "--wait", "--wait-timeout=180", "server", timeout=210)
        registered = self.register(refresh=True)
        core, database = registered["coreContainerId"], registered["databaseContainerId"]
        before = self.json_command("inspect", database)[0]
        keys = self.keys(core)
        token = self.login(admin_password)
        self.http("resourceBindings/secrets", {"name": "upgrade-fixture", "value": secrets.token_hex(32)}, token)
        ciphertext = self.sql(database, "SELECT encryptedvalue FROM internalsecretvalues ORDER BY secretid")
        require(ciphertext, "Encrypted fixture credential was not persisted")
        status = self.http("updates/check", {}, token)
        require((status.get("available") or {}).get("version") == target_version,
                "Target is not the currently offered release; no update was approved")
        review = self.http("updates/review", {"version": target_version}, token)
        require(review["targetVersion"] == target_version and review["installedVersion"] == source["targetVersion"],
                "Review selected a different transition")
        self.approval_attempted = True
        operation = self.http("updates/approve", {"reviewId": review["id"], "requestId": str(uuid.uuid4())}, token)
        operation_id = str(uuid.UUID(operation["id"]))
        print("Update approved once; waiting for verified completion", flush=True)
        deadline = time.monotonic() + 1200
        refresh_at = time.monotonic() + 60
        while time.monotonic() < deadline:
            # Renew through normal login, without flooding authentication while
            # Core is offline or maintenance blocks new sessions.
            if time.monotonic() >= refresh_at:
                token = self.login(admin_password, optional=True) or token
                refresh_at = time.monotonic() + 60
            status = self.http("updates", token=token, optional=True)
            outcome = self.http("updates/operations/" + operation_id, token=token, optional=True)
            if status and outcome:
                require(outcome["phase"] not in ("Failed", "Postponed", "RecoveryRequired"),
                        "Updater reported a terminal failure; transition did not pass")
                if outcome["phase"] == "Succeeded" and status.get("operation") is None:
                    require(status["installedVersion"] == target_version, "Unexpected running version")
                    break
            time.sleep(3)
        else:
            raise AssertionError("Update did not complete within the fixture deadline")
        self.login(admin_password)
        after = self.json_command("inspect", database)[0]
        require(database_state(before) == database_state(after), "Update restarted or replaced PostgreSQL")
        refreshed = json.loads(self.read(self.state + "/installation.json"))
        require_preserved(registered, refreshed)
        require(self.keys(refreshed["coreContainerId"]) == keys, "Update changed recovery keys")
        require(self.sql(database, "SELECT encryptedvalue FROM internalsecretvalues ORDER BY secretid") == ciphertext,
                "Update changed the encrypted fixture credential")
        checked = self.utility("check-update-installation")
        require(checked["database"]["instanceId"] == registered["instanceId"], "Instance identity changed")
        self.restore_checkpoint(operation_id, registered, password, ciphertext)
        print("Published transition and fresh-database checkpoint recovery passed", flush=True)

    def capture_failure(self):
        if not getattr(self, "approval_attempted", False):
            return
        # Private fixture credentials/journals never enter the checkout or stdout.
        destination = Path(tempfile.mkdtemp(prefix="citadel-upgrade-failure-"))
        copied = self.host("cp", f"{self.daemon}:{self.state}", str(destination / "state"), check=False)
        containers = self.docker("ps", "-aq", check=False).stdout.decode().split()
        for container in containers:
            result = self.docker("logs", "--tail=500", container, check=False)
            (destination / (container + ".log")).write_bytes(result.stdout + result.stderr)
        print(f"Private failure logs retained at {destination}; state copied: {copied.returncode == 0}", flush=True)

    def restore_checkpoint(self, operation_id, registered, password, ciphertext):
        checkpoint = self.state + "/checkpoint-" + operation_id
        receipt = json.loads(self.read(checkpoint + "/checkpoint.json"))
        require(receipt["operationId"] == operation_id and receipt["instanceId"] == registered["instanceId"],
                "Checkpoint belongs to a different operation or instance")
        # Keep the original database and host evidence intact during recovery.
        self.compose("stop", "server", timeout=120)
        preserved = self.execute("sh", "-c", 'find "$1" -type f -exec sha256sum {} + | sort',
                                 "sh", self.state).stdout
        restored = self.docker("run", "-d", "--network=none", "--tmpfs=/var/lib/postgresql",
                               "-e", "POSTGRES_USER=citadel", "-e", "POSTGRES_DB=citadel",
                               "-e", "POSTGRES_PASSWORD=" + password, registration.POSTGRES).stdout.decode().strip()
        deadline = time.monotonic() + 60
        while self.docker("exec", restored, "pg_isready", "-h", "127.0.0.1", "-U", "citadel", "-d", "citadel", check=False).returncode:
            require(time.monotonic() < deadline, "Restore database did not become ready")
            time.sleep(1)
        self.docker("run", "--rm", "--network=container:" + restored, "--user=0:0",
                    "--entrypoint=/app/citadel-server",
                    "--mount", f"type=bind,src={self.state},dst={self.state},readonly",
                    "-e", "PG_HOST=127.0.0.1", "-e", "PG_USER=citadel", "-e", "PG_DATABASE=citadel",
                    "-e", "PG_PASSWORD=" + password, "-e", "CITADEL_DATA_ROOT=" + receipt["bundle"] + "/recovery",
                    self.core, "restore-update-checkpoint", "--checkpoint-directory", checkpoint,
                    "--confirm-instance-replacement", timeout=300)
        require(self.sql(restored, "SELECT instanceid FROM citadelinstanceidentity") == registered["instanceId"].encode(),
                "Restored instance identity differs")
        require(self.sql(restored, "SELECT encryptedvalue FROM internalsecretvalues ORDER BY secretid") == ciphertext,
                "Restored credential differs")
        require(self.sql(restored, "SELECT count(*) FROM instanceupdates WHERE ownsinstallation") == b"0",
                "Recovery did not release restored maintenance")
        require(self.sql(restored, "SELECT operation->>'phase' FROM instanceupdates") == b"Failed",
                "Recovery did not record the failed update")
        require(self.sql(registered["databaseContainerId"], "SELECT operation->>'phase' FROM instanceupdates") == b"Succeeded",
                "Recovery changed the original update outcome")
        require(self.execute("sh", "-c", 'find "$1" -type f -exec sha256sum {} + | sort',
                             "sh", self.state).stdout == preserved, "Recovery changed host evidence")


def require_eligible(source, target, architecture):
    candidates = [s for s in target["descriptor"]["sources"]
                  if s["version"] == source["targetVersion"]
                  and s["catalogSha256"] == source["targetCatalogSha256"]
                  and architecture in s["architectures"]]
    require(len(candidates) == 1, "Signed target does not allow this source/catalog/architecture; nothing was installed")


def database_state(container):
    return container["Id"], container["State"]["StartedAt"], container["RestartCount"]


def require_preserved(before, after):
    mutable = {"coreContainerId", "configurationSha256", "runtimeSha256"}
    require({k: v for k, v in before.items() if k not in mutable}
            == {k: v for k, v in after.items() if k not in mutable}, "Installation/storage identity changed")
    require(before["coreContainerId"] != after["coreContainerId"], "Core was not replaced")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-image", required=True, help="Exact published ghcr.io/citadel-p/citadel@sha256:... digest")
    parser.add_argument("--target-version", required=True, help="Exact currently offered published version")
    args = parser.parse_args()
    def interrupted(*_):
        raise KeyboardInterrupt()
    signal.signal(signal.SIGTERM, interrupted)
    with tempfile.TemporaryDirectory(prefix="citadel-upgrade-acceptance-") as temporary:
        fixture = UpgradeFixture(args.source_image, Path(temporary))
        try:
            fixture.start(published=True)
            fixture.run_transition(args.target_version)
        except BaseException:
            try:
                fixture.capture_failure()
            except Exception:
                print("Could not capture all private fixture evidence", flush=True)
            raise
        finally:
            fixture.close()


if __name__ == "__main__":
    main()
