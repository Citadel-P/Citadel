#!/usr/bin/env python3
"""Run the real updater lifecycle with a test-only release catalog, without publication.

The test executable is never shipped. Docker, PostgreSQL, HTTP authentication,
inspection, the detached helper, target acceptance and recovery are real. The
private registry impersonates ghcr.io only inside the disposable DinD fixture.
Browser mode retains its network and exposes only the UI on localhost.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time

spec = importlib.util.spec_from_file_location(
    "upgrade", Path(__file__).with_name("test-instance-upgrade.py"))
upgrade = importlib.util.module_from_spec(spec)
spec.loader.exec_module(upgrade)
registration = upgrade.registration
require = upgrade.require
SOURCE = "0.1.1-dev.9001"
TARGET = "0.1.1-dev.9002"


def build(base, version, directory, catalog, frontend=None):
    """Only build the explicitly test-only executable, using existing Cargo cache."""
    print(f"Building private lifecycle fixture {version}", flush=True)
    env = dict(os.environ, CITADEL_VERSION=version, CITADEL_INFORMATIONAL_VERSION=version,
               SQLX_OFFLINE="true")
    result = subprocess.run([
        "cargo", "test", "--locked", "-p", "citadel-server", "--test",
        "instance_upgrade_fixture", "--no-run", "--message-format=json",
    ], cwd=registration.ROOT, env=env, capture_output=True, timeout=1800)
    if result.returncode:
        log = Path(tempfile.mkdtemp(prefix="citadel-upgrade-build-failure-")) / "build.log"
        log.write_bytes(result.stdout + result.stderr)
        raise RuntimeError(f"Fixture compilation failed; see {log}")
    require(result.returncode == 0, f"Fixture compilation failed; see {directory}")
    artifacts = [json.loads(line) for line in result.stdout.splitlines() if line.startswith(b"{")]
    executable = next(item["executable"] for item in artifacts
                      if item.get("reason") == "compiler-artifact"
                      and item["target"]["name"] == "instance_upgrade_fixture" and item.get("executable"))
    context = directory / version
    context.mkdir()
    shutil.copy2(executable, context / "citadel-server")
    if frontend:
        shutil.copytree(frontend, context / "wwwroot")
    fixture = dict(target=dict(version=TARGET, image="ghcr.io/citadel-p/citadel@sha256:" + "0" * 64),
                   sources=[dict(version=SOURCE, catalogSha256=catalog,
                                 architectures=["amd64", "arm64"], agentVersions=[])])
    (context / "upgrade-fixture.json").write_text(json.dumps(fixture))
    (context / "Dockerfile").write_text(
        f"FROM {base}\nCOPY --chmod=755 citadel-server /app/citadel-server\n"
        "COPY --chmod=644 upgrade-fixture.json /app/upgrade-fixture.json\n"
        + ("COPY --chown=65532:0 wwwroot /app/wwwroot\nENV CITADEL_STATIC_ROOT=/app/wwwroot\n" if frontend else "") +
        "LABEL com.citadel.qualification-only=true\n")
    image = "citadel-upgrade-fixture:" + version + "-" + directory.name
    registration.run([*registration.DOCKER, "build", "--builder", "default", "--tag", image, str(context)], timeout=300)
    return image


class LocalUpgrade(upgrade.UpgradeFixture):
    interrupt_cleanup = False

    def serve_browser(self, port):
        self.inspect_transition(TARGET)
        _, _, password = self.setup_installation(browser_port=port)
        # Leave discovery, review and approval to the person using the browser.
        print(f"\nOpen http://localhost:{port}/updates\n"
              f"Email: upgrade@example.test\nPassword: {password}\n\n"
              f"Check now → Update now → Update and restart ({SOURCE} → {TARGET}).\n"
              "Keep this process running. Ctrl+C removes this disposable installation.\n",
              flush=True)
        while True:
            time.sleep(1)

    def after_approval(self, operation_id, registered):
        if not self.interrupt_cleanup:
            return
        checkpoint = self.state + "/checkpoint-" + operation_id
        deadline = time.monotonic() + 600
        while self.execute("test", "-f", checkpoint + "/checkpoint.json", check=False).returncode:
            require(time.monotonic() < deadline, "Update did not create its checkpoint")
            time.sleep(0.25)
        # Install the fault after the dump, so recovery never restores the test
        # trigger. Pause the final DB release while killing the actual target.
        database = registered["databaseContainerId"]
        self.sql(database, """
                CREATE FUNCTION fixture_pause_release() RETURNS trigger LANGUAGE plpgsql AS $$
                BEGIN
                    IF OLD.ownsinstallation AND NOT NEW.ownsinstallation THEN
                        PERFORM pg_sleep(30);
                    END IF;
                    RETURN NEW;
                END $$;
                CREATE TRIGGER fixture_pause_release BEFORE UPDATE ON instanceupdates
                FOR EACH ROW EXECUTE FUNCTION fixture_pause_release();
        """)
        completed = checkpoint + "/completed.json"
        while self.execute("test", "-f", completed, check=False).returncode:
            require(time.monotonic() < deadline, "Update did not reach final cleanup")
            time.sleep(0.25)
        target = self.compose("ps", "-q", "server").stdout.decode().strip()
        require(target and target != registered["coreContainerId"], "Replacement Core is missing")
        policy = self.json_command("inspect", target)[0]["HostConfig"]["RestartPolicy"]["Name"]
        self.docker("update", "--restart=no", target)
        self.docker("kill", "--signal=SIGKILL", target)
        self.sql(database, "DROP TRIGGER fixture_pause_release ON instanceupdates; DROP FUNCTION fixture_pause_release()")
        require(self.sql(database, "SELECT count(*) FROM instanceupdates WHERE ownsinstallation AND operation->>'phase'='Succeeded'") == b"1",
                "Fixture did not interrupt the retained successful claim")
        self.docker("update", "--restart=" + policy, target)
        self.docker("start", target)
        print("Interrupted final release; verifying restart resumes cleanup without replacement", flush=True)

    def load(self, image):
        archive = self.directory / "target.tar"
        self.host("save", "--output", str(archive), image, timeout=300)
        with archive.open("rb") as stream:
            self.execute("docker", "load", stdin=stream, timeout=300)
        archive.unlink()

    def publish_private(self, image, tag):
        name = "ghcr.io/citadel-p/citadel:" + tag
        self.docker("tag", image, name)
        self.docker("push", name, timeout=300)
        digests = self.json_command("image", "inspect", name)[0]["RepoDigests"]
        digest = next((d for d in digests if d.startswith("ghcr.io/citadel-p/citadel@sha256:")), "")
        require(digest.startswith("ghcr.io/citadel-p/citadel@sha256:"), "Unexpected private digest")
        return digest

    def prepare_images(self, target):
        self.load(target)
        self.docker("run", "-d", "--network=host", "-e", "REGISTRY_HTTP_ADDR=0.0.0.0:80",
                    registration.REGISTRY)
        deadline = time.monotonic() + 30
        while self.execute("wget", "-qO-", "http://127.0.0.1/v2/", check=False).returncode:
            require(time.monotonic() < deadline, "Private registry did not become ready")
            time.sleep(1)
        target = self.publish_private(target, "target")
        # Only the private source image needs the final target digest. The target
        # descriptor carries the source version/catalog, avoiding circular hashes.
        self.execute("mkdir", "-p", "/opt/fixture-build")
        config = json.loads(self.docker("run", "--rm", "--network=none", "--entrypoint=cat",
                                       self.core, "/app/upgrade-fixture.json").stdout)
        config["target"]["image"] = target
        self.write("/opt/fixture-build/upgrade-fixture.json", json.dumps(config))
        self.write("/opt/fixture-build/Dockerfile",
                   f"FROM {self.core}\nCOPY --chmod=644 upgrade-fixture.json /app/upgrade-fixture.json\n")
        self.docker("build", "--tag", "citadel-fixture-source", "/opt/fixture-build", timeout=300)
        self.core = self.publish_private("citadel-fixture-source", "source")

    def inspect_transition(self, target_version):
        identity = self.json_command("run", "--rm", "--network=none", self.core, "version-json")
        require(identity.get("qualificationBuild") is True, "Expected the test-only executable")
        source = self.json_command("run", "--rm", "--network=none", self.core, "upgrade-descriptor-json")
        architecture = self.json_command("image", "inspect", self.core)[0]["Architecture"]
        target = self.json_command(
            "run", "--rm", "--mount", "type=bind,src=/var/run/docker.sock,dst=/var/run/docker.sock",
            self.core, "inspect-update-release", "--version", target_version, "--architecture", architecture)
        upgrade.require_eligible(source, target, architecture)
        return source


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime-image", required=True, help="Locally built Core runtime image")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--browser", action="store_true", help="Keep an isolated installation open for manual UI testing")
    mode.add_argument("--scenario", choices=["all", "upgrade", "cleanup-restart"], default="all",
                        help="Run normal upgrade and interrupted cleanup by default")
    parser.add_argument("--port", type=int, default=18080, help="Localhost port for --browser (default: 18080)")
    args = parser.parse_args()
    if not 1 <= args.port <= 65535:
        parser.error("--port must be between 1 and 65535")
    def interrupted(*_):
        raise KeyboardInterrupt()
    signal.signal(signal.SIGTERM, interrupted)
    import hashlib
    catalog = hashlib.sha256((registration.ROOT / "src/infrastructure/database/generated/migrations.json").read_bytes()).hexdigest()
    images = {}
    with tempfile.TemporaryDirectory(prefix="citadel-local-upgrade-") as temporary:
        directory = Path(temporary)
        fixture = None
        try:
            frontend = None
            if args.browser:
                print("Building the current frontend for browser testing", flush=True)
                subprocess.run(["npm", "run", "build:image", "--prefix", "src/frontend"],
                    cwd=registration.ROOT, env=dict(os.environ, VITE_API_BASE_URL=""), check=True)
                frontend = registration.ROOT / "src/frontend/dist"
            for version in [TARGET, SOURCE]:
                images[version] = build(args.runtime_image, version, directory, catalog, frontend)
            if args.browser:
                fixture = LocalUpgrade(images[SOURCE], directory)
                fixture.start(registry=True, browser_port=args.port)
                fixture.prepare_images(images[TARGET])
                fixture.serve_browser(args.port)
                return
            scenarios = ["upgrade", "cleanup-restart"] if args.scenario == "all" else [args.scenario]
            for scenario in scenarios:
                print(f"Running lifecycle scenario: {scenario}", flush=True)
                fixture = LocalUpgrade(images[SOURCE], directory)
                fixture.interrupt_cleanup = scenario == "cleanup-restart"
                fixture.start(registry=True)
                fixture.prepare_images(images[TARGET])
                fixture.run_transition(TARGET)
                fixture.close()
                fixture = None
        except BaseException as error:
            if args.browser and isinstance(error, KeyboardInterrupt):
                print("\nRemoving browser test installation", flush=True)
            else:
                if fixture and fixture.daemon:
                    # Preserve private logs even when setup or registration fails.
                    fixture.approval_attempted = True
                    fixture.capture_failure()
                raise
        finally:
            if fixture:
                fixture.close()
            for image in images.values():
                registration.run([*registration.DOCKER, "image", "rm", image], check=False)


if __name__ == "__main__":
    main()
