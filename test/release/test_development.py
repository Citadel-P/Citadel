"""Development-channel isolation, publication recovery and VPS deployment guards."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from test_release import FakeBackend, ROOT, metadata, record, release


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / f"src/tools/release/{name}.py")
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {"release": release}):
        spec.loader.exec_module(module)
    return module


demo, remote = load("demo"), load("demo_remote")


def dev_record(number=1):
    value = record()
    data = value["metadata"]
    data.update(displayVersion=f"1.2.3-dev.{number}", releaseEligible=False,
                developmentReleaseEligible=True, publicationEligible=True)
    data["informationalVersion"] = data["displayVersion"] + "+height.5.sha." + "a" * 40
    value.update(tag="v" + data["displayVersion"], compatibility=release.development_compatibility())
    return value


class DevelopmentTests(unittest.TestCase):
    def test_preflight_does_not_require_stable_hosting_or_baselines(self):
        value = dev_record()
        backend = FakeBackend()
        env = {"CITADEL_RUST_AGENT_RELEASE_ENABLED": "true"}
        self.assertEqual(release.preflight(value["metadata"], backend, env), release.development_compatibility())
        with self.assertRaises(ValueError):
            release.preflight(metadata(), backend, env)
        for change in [dict(releaseEligible=True), dict(displayVersion="1.2.4-dev.1"), dict(dirty=True)]:
            with self.assertRaises(ValueError):
                release.check_metadata(dict(value["metadata"], **change))

    def test_ghcr_only_dev_aliases_numeric_order_and_partial_retry(self):
        backend = FakeBackend()
        value = dev_record(2)
        backend.current = value
        backend.fail = ("copy", "ghcr.io/citadel-p/citadel:1.2.3-dev.2")
        with self.assertRaises(RuntimeError):
            release.promote(backend, value, Path("/candidates"))
        self.assertFalse(any(tag == "dev" for _, tag in backend.images))
        backend.fail = None
        release.promote(backend, value, Path("/candidates"))
        self.assertTrue(all(repo.startswith("ghcr.io/") for repo, _ in backend.images))
        self.assertEqual({tag for _, tag in backend.images}, {"1.2.3-dev.2", "dev"})
        newer = dev_record(10)
        newer["indexes"] = {c: "sha256:" + "f" * 64 for c in ["core", "agent"]}
        backend.current = newer
        release.promote(backend, newer, Path("/candidates"))
        backend.current = value
        backend.operations = []
        release.promote(backend, value, Path("/candidates"))
        self.assertFalse(any(op[0] == "copy" for op in backend.operations))
        self.assertEqual(backend.images["ghcr.io/citadel-p/citadel", "dev"], newer["indexes"]["core"])

    def test_finalization_marks_prerelease_without_pages(self):
        backend, value = FakeBackend(), dev_record()
        value["status"] = "images-verified"
        backend.records[value["tag"]] = value
        env = dict(CITADEL_RUST_AGENT_RELEASE_ENABLED="true", GITHUB_EVENT_NAME="push",
                   GITHUB_REF="refs/tags/" + value["tag"], GITHUB_SHA="a" * 40)
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "metadata.json"
            path.write_text(json.dumps(value["metadata"]))
            argv = ["release.py", "finalize", "--apply", "--metadata", str(path),
                    "--directory", temp, "--repository", backend.repository]
            with patch.dict(release.os.environ, env, clear=True), patch.object(release.sys, "argv", argv), \
                 patch.object(release, "Backend", return_value=backend), \
                 patch.object(release.version, "git", side_effect=["tag", "a" * 40]), \
                 patch.object(backend, "command", create=True) as command:
                release.main()
            self.assertIn("--prerelease", command.call_args.args)
            self.assertIn("--latest=false", command.call_args.args)
            self.assertEqual(backend.load(value["tag"])["status"], "complete")

    def test_demo_only_consumes_completed_signed_prerelease_digests(self):
        backend, value = FakeBackend(), dev_record()
        backend.current = value
        release.promote(backend, value, Path("/candidates"))
        with self.assertRaisesRegex(ValueError, "completed"):
            demo.deployment_payload(value["metadata"], backend)
        value["status"] = "complete"
        backend.save(value)
        with patch.object(backend, "release", return_value={"draft": False, "prerelease": True}):
            payload = demo.deployment_payload(value["metadata"], backend)
            self.assertEqual(payload["core"], "ghcr.io/citadel-p/citadel@" + value["indexes"]["core"])
            value["steps"].pop("signed:ghcr.io/citadel-p/citadel:1.2.3-dev.1")
            backend.save(value)
            with self.assertRaisesRegex(ValueError, "signature"):
                demo.deployment_payload(value["metadata"], backend)


class DemoTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        (self.directory / ".env").write_text("PG_PASSWORD=private\n")
        self.payload = dict(version="1.2.3-dev.10", core="ghcr.io/citadel-p/citadel@sha256:" + "a" * 64,
                            agent="ghcr.io/citadel-p/citadel.agent@sha256:" + "b" * 64, compose="services: {}\n")

    def test_deploy_pins_both_images_and_preserves_env_and_newer_version(self):
        with patch.object(remote.subprocess, "run") as run:
            self.assertEqual(remote.deploy(self.directory, self.payload)["status"], "deployed")
            command = run.call_args.args[0]
            self.assertIn("--wait", command)
            self.assertEqual(run.call_args.kwargs["env"]["CITADEL_IMAGE"], self.payload["core"])
            self.assertEqual((self.directory / ".env").read_text(), "PG_PASSWORD=private\n")
            run.reset_mock()
            older = dict(self.payload, version="1.2.3-dev.2")
            self.assertEqual(remote.deploy(self.directory, older)["status"], "skipped-older-release")
            run.assert_not_called()
            with self.assertRaisesRegex(ValueError, "different image"):
                remote.deploy(self.directory, dict(self.payload, core=self.payload["core"].replace("a" * 64, "f" * 64)))

    def test_failed_upgrade_blocks_older_deploy_but_allows_same_version_retry(self):
        def fail_up(args, **kwargs):
            if "up" in args:
                raise subprocess.CalledProcessError(1, args)
        with patch.object(remote.subprocess, "run", side_effect=fail_up):
            with self.assertRaises(subprocess.CalledProcessError):
                remote.deploy(self.directory, self.payload)
        self.assertFalse((self.directory / "deployment.json").exists())
        with patch.object(remote.subprocess, "run") as run:
            self.assertEqual(remote.deploy(self.directory, dict(self.payload, version="1.2.3-dev.2"))["status"], "skipped-older-release")
            run.assert_not_called()
            self.assertEqual(remote.deploy(self.directory, self.payload)["status"], "deployed")

    def test_rejects_floating_images_and_unsafe_ssh_settings(self):
        with patch.object(remote.subprocess, "run") as run:
            with self.assertRaises(ValueError):
                remote.deploy(self.directory, dict(self.payload, core="ghcr.io/citadel-p/citadel:dev"))
            run.assert_not_called()
        env = dict(CITADEL_DEMO_HOST="demo.example.com", CITADEL_DEMO_USER="citadel",
                   CITADEL_DEMO_DIRECTORY="/opt/citadel-demo", CITADEL_DEMO_URL="https://demo.example.com")
        self.assertEqual(demo.settings(env)[2], "22")
        for change in [dict(CITADEL_DEMO_HOST="-oProxyCommand=bad"), dict(CITADEL_DEMO_USER="user;bad"),
                       dict(CITADEL_DEMO_PORT="0"), dict(CITADEL_DEMO_URL="http://demo.example.com"),
                       dict(CITADEL_DEMO_DIRECTORY="/")]:
            with self.assertRaises(ValueError):
                demo.settings(dict(env, **change))

    def test_demo_dry_run_and_manual_context_cannot_write(self):
        value = dev_record()
        metadata_file = self.directory / "version.json"
        metadata_file.write_text(json.dumps(value["metadata"]))
        argv = ["demo.py", "--repository", "Citadel-P/Citadel", "--metadata", str(metadata_file),
                "--identity", "/unused/key", "--known-hosts", "/unused/known_hosts"]
        env = dict(CITADEL_DEMO_HOST="demo.example.com", CITADEL_DEMO_USER="citadel",
                   CITADEL_DEMO_DIRECTORY="/opt/citadel-demo", CITADEL_DEMO_URL="https://demo.example.com",
                   GITHUB_EVENT_NAME="workflow_dispatch", GITHUB_REF="refs/tags/" + value["tag"],
                   GITHUB_SHA=value["metadata"]["sourceRevision"])
        with patch.dict(demo.os.environ, env, clear=True), patch.object(demo.sys, "argv", argv), \
             patch.object(demo, "deployment_payload", return_value=self.payload), \
             patch.object(demo.subprocess, "run") as run, patch("builtins.print") as output:
            demo.main()
            self.assertTrue(json.loads(output.call_args.args[0])["dryRun"])
            run.assert_not_called()
            argv.append("--apply")
            with self.assertRaisesRegex(ValueError, "tag push context"):
                demo.main()
            run.assert_not_called()

    def test_failed_https_health_check_fails_job(self):
        with patch.object(demo.urllib.request, "urlopen", side_effect=OSError("unreachable")), \
             patch.object(demo.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "health check failed"):
                demo.verify_health("https://demo.example.com")


if __name__ == "__main__":
    unittest.main()
