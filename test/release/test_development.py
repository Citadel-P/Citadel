"""Development-channel isolation, publication recovery and VPS deployment guards."""
import copy
import importlib.util
import io
import json
import re
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
    def test_preflight_requires_docs_but_not_stable_registries_or_baselines(self):
        value = dev_record()
        backend = FakeBackend()
        env = {"CITADEL_RUST_AGENT_RELEASE_ENABLED": "true", "DOCS_SITE_URL": "https://docs.example.invalid"}
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

    def test_finalization_marks_prerelease_after_pages(self):
        backend, value = FakeBackend(), dev_record()
        value["status"] = "images-verified"
        backend.records[value["tag"]] = value
        env = dict(CITADEL_RUST_AGENT_RELEASE_ENABLED="true", GITHUB_EVENT_NAME="push",
                   GITHUB_REF="refs/tags/" + value["tag"], GITHUB_SHA="a" * 40,
                   DOCS_SITE_URL="https://docs.example.invalid", CITADEL_DOCS_RESULT="https://docs.example.invalid/")
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
            self.assertEqual(backend.load(value["tag"])["steps"]["documentation"], env["CITADEL_DOCS_RESULT"])

    def test_demo_only_consumes_completed_signed_prerelease_digests(self):
        backend, value = FakeBackend(), dev_record()
        backend.current = value
        release.promote(backend, value, Path("/candidates"))
        with self.assertRaisesRegex(ValueError, "completed"):
            demo.deployment_payload(value["metadata"], backend, "dev")
        value["status"] = "complete"
        backend.save(value)
        with patch.object(backend, "release", return_value={"draft": False, "prerelease": True}):
            payload = demo.deployment_payload(value["metadata"], backend, "dev")
            self.assertEqual(payload["core"], "ghcr.io/citadel-p/citadel@" + value["indexes"]["core"])
            value["steps"].pop("signed:ghcr.io/citadel-p/citadel:1.2.3-dev.1")
            backend.save(value)
            with self.assertRaisesRegex(ValueError, "signature"):
                demo.deployment_payload(value["metadata"], backend, "dev")


class DeploymentPayloadTests(unittest.TestCase):
    def published(self, channel):
        backend = FakeBackend()
        value = dev_record() if channel == "dev" else record()
        value["status"] = "complete"
        for component, repositories in release.repositories(value["metadata"]).items():
            repo = repositories[0]
            value["steps"]["signed:" + repo + ":" + value["metadata"]["displayVersion"]] = value["indexes"][component]
            if channel == "dev" or component == "core":
                backend.images[repo, channel] = value["indexes"][component]
        backend.save(value)
        return backend, value, {"draft": False, "prerelease": channel == "dev"}

    def test_completed_channels_use_coordinated_digests_without_agent_latest(self):
        for channel in ("dev", "latest"):
            with self.subTest(channel=channel):
                backend, value, published = self.published(channel)
                with patch.object(backend, "release", return_value=published):
                    payload = demo.deployment_payload(value["metadata"], backend, channel)
                self.assertEqual(payload["channel"], channel)
                for component in ("core", "agent"):
                    self.assertTrue(payload[component].endswith("@" + value["indexes"][component]))
                self.assertEqual(payload["compose"], (ROOT / "deploy/install/docker-compose.yml").read_text())
                self.assertNotIn(("ghcr.io/citadel-p/citadel.agent", "latest"), backend.images)

    def test_rejects_unpublished_mismatched_incomplete_or_unsigned_records(self):
        for channel in ("dev", "latest"):
            for case in ("draft", "wrong-prerelease", "missing-release", "incomplete", "missing-record",
                         "metadata", "unsigned-core", "unsigned-agent", "bad-digest"):
                with self.subTest(channel=channel, case=case):
                    backend, value, published = self.published(channel)
                    expected = copy.deepcopy(value["metadata"])
                    if case == "draft": published["draft"] = True
                    if case == "wrong-prerelease": published["prerelease"] = not published["prerelease"]
                    if case == "missing-release": published = None
                    if case == "incomplete": value["status"] = "images-verified"
                    if case == "metadata": value["metadata"]["sourceRevision"] = "b" * 40
                    if case.startswith("unsigned-"):
                        component = case.removeprefix("unsigned-")
                        repo = release.repositories(expected)[component][0]
                        del value["steps"]["signed:" + repo + ":" + expected["displayVersion"]]
                    if case == "bad-digest": value["indexes"]["core"] = "invalid"
                    backend.save(value)
                    if case == "missing-record": backend.records.clear()
                    with patch.object(backend, "release", return_value=published), self.assertRaises(ValueError):
                        demo.deployment_payload(expected, backend, channel)

    def test_changed_alias_skips_but_missing_alias_or_registry_failure_fails(self):
        for channel, component in (("dev", "core"), ("dev", "agent"), ("latest", "core")):
            with self.subTest(channel=channel, component=component):
                backend, value, published = self.published(channel)
                repo = release.repositories(value["metadata"])[component][0]
                with patch.object(backend, "release", return_value=published):
                    backend.images[repo, channel] = "sha256:" + "f" * 64
                    result = demo.deployment_payload(value["metadata"], backend, channel)
                    self.assertEqual(result["status"], "skipped-stale-alias")
                    self.assertNotIn("compose", result)
                    del backend.images[repo, channel]
                    with self.assertRaisesRegex(ValueError, "alias"):
                        demo.deployment_payload(value["metadata"], backend, channel)
                    with patch.object(backend, "existing", side_effect=RuntimeError("denied")), self.assertRaises(RuntimeError):
                        demo.deployment_payload(value["metadata"], backend, channel)

    def test_channel_must_match_release_type(self):
        for channel, value in (("dev", record()), ("latest", dev_record()), ("other", dev_record())):
            with self.subTest(channel=channel), self.assertRaises(ValueError):
                demo.deployment_payload(value["metadata"], FakeBackend(), channel)


class DemoTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        (self.directory / ".env").write_text("PG_PASSWORD=private\n")
        self.payload = dict(channel="dev", version="1.2.3-dev.10", core="ghcr.io/citadel-p/citadel@sha256:" + "a" * 64,
                            agent="ghcr.io/citadel-p/citadel.agent@sha256:" + "b" * 64, compose="services: {}\n")

    def test_deploy_pins_both_images_and_preserves_env_and_newer_version(self):
        with patch.object(remote.subprocess, "run") as run:
            self.assertEqual(remote.deploy(self.directory, self.payload)["status"], "deployed")
            command = run.call_args.args[0]
            self.assertIn("--wait", command)
            self.assertEqual(command[2:6], ["--env-file", ".env", "--env-file", ".release.env"])
            self.assertIn(self.payload["agent"], (self.directory / ".release.env").read_text())
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
        argv = ["demo.py", "--channel", "dev", "--repository", "Citadel-P/Citadel", "--metadata", str(metadata_file),
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

    def test_remote_channel_and_version_validation(self):
        for channel, version in (("dev", "1.2.3-dev.10"), ("latest", "1.2.3")):
            remote.validate(dict(self.payload, channel=channel, version=version))
        for change in (dict(channel="latest"), dict(version="1.2.3"), dict(channel="other"),
                       dict(version="1.2.3-dev.01"), dict(version="1.2.3-dev.0"),
                       dict(channel="latest", version="1.2.3-rc.1"), dict(version="1.2.3-dev.x"),
                       dict(core="ghcr.io/citadel-p/citadel:latest"),
                       dict(agent="ghcr.io/citadel-p/citadel.agent:dev"),
                       dict(core="ghcr.io/other/citadel@sha256:" + "a" * 64)):
            with self.subTest(change=change), self.assertRaises(ValueError):
                remote.validate(dict(self.payload, **change))

    def test_stable_ordering_and_channel_pinning(self):
        stable = dict(self.payload, channel="latest", version="1.10.0")
        self.assertGreater(remote.order("1.2.3-dev.10", "dev"), remote.order("1.2.3-dev.9", "dev"))
        self.assertGreater(remote.order("1.10.0", "latest"), remote.order("1.9.9", "latest"))
        with patch.object(remote.subprocess, "run") as run:
            remote.deploy(self.directory, stable)
            self.assertEqual((self.directory / ".deployment-channel").read_text(), "latest\n")
            run.reset_mock()
            self.assertEqual(remote.deploy(self.directory, dict(stable, version="1.9.9"))["status"], "skipped-older-release")
            run.assert_not_called()
            with self.assertRaisesRegex(ValueError, "pinned to channel"):
                remote.deploy(self.directory, self.payload)
            run.assert_not_called()

    def test_invalid_channel_guard_blocks_before_compose_mutation(self):
        for content in ("", "invalid\n", "dev\n\n", " dev\n"):
            (self.directory / ".deployment-channel").write_text(content)
            with self.subTest(content=content), patch.object(remote.subprocess, "run") as run:
                with self.assertRaisesRegex(ValueError, "Invalid .deployment-channel"):
                    remote.deploy(self.directory, self.payload)
                run.assert_not_called()
                self.assertFalse((self.directory / "docker-compose.yml").exists())

    def test_legacy_dev_attempt_is_preserved_and_cannot_initialize_stable(self):
        legacy = {k: self.payload[k] for k in ("version", "core", "agent")}
        path = self.directory / "deployment-attempt.json"
        path.write_text(json.dumps(legacy))
        with patch.object(remote.subprocess, "run") as run:
            with self.assertRaisesRegex(ValueError, "pinned to channel"):
                remote.deploy(self.directory, dict(self.payload, channel="latest", version="1.2.3"))
            self.assertEqual(json.loads(path.read_text()), legacy)
            self.assertFalse((self.directory / ".deployment-channel").exists())
            self.assertEqual(remote.deploy(self.directory, dict(self.payload, version="1.2.3-dev.9"))["status"], "skipped-older-release")
            run.assert_not_called()
        self.assertEqual(json.loads(path.read_text()), {"channel": "dev", **legacy})

    def test_inconsistent_history_cannot_reset_attempt_guard(self):
        identity = {k: self.payload[k] for k in remote.IDENTITY}
        attempt = self.directory / "deployment-attempt.json"
        completed = self.directory / "deployment.json"
        for value in ({"bad": "state"}, dict(identity, version="1.2.3-dev.11"),
                      dict(identity, core=identity["core"].replace("a" * 64, "c" * 64)),
                      dict(identity, channel="latest", version="1.2.3")):
            attempt.write_text(json.dumps(identity))
            completed.write_text(json.dumps(value))
            with self.subTest(value=value), patch.object(remote.subprocess, "run") as run:
                with self.assertRaises(ValueError):
                    remote.deploy(self.directory, self.payload)
                run.assert_not_called()
                self.assertFalse((self.directory / ".deployment-channel").exists())
                self.assertEqual(json.loads(attempt.read_text()), identity)

    def test_compose_failure_before_up_does_not_advance_attempt(self):
        with patch.object(remote.subprocess, "run", side_effect=subprocess.CalledProcessError(1, ["docker"])):
            with self.assertRaises(subprocess.CalledProcessError):
                remote.deploy(self.directory, self.payload)
        self.assertFalse((self.directory / "deployment-attempt.json").exists())
        self.assertEqual((self.directory / ".env").read_text(), "PG_PASSWORD=private\n")

    def test_lock_wait_is_bounded(self):
        with patch.object(remote.fcntl, "flock", side_effect=BlockingIOError), \
             patch.object(remote.time, "monotonic", side_effect=[0, 121]), self.assertRaises(TimeoutError):
            remote.acquire_lock(object())

    def test_public_health_requires_ok_object(self):
        for data, accepted in (({"status": "ok"}, True), ({"status": "error"}, False), ([], False)):
            response = io.StringIO(json.dumps(data))
            response.status = 200
            with self.subTest(data=data), patch.object(demo.urllib.request, "urlopen", return_value=response) as request, \
                 patch.object(demo.time, "sleep"):
                if accepted:
                    demo.verify_health("https://demo.example.com")
                    self.assertEqual(request.call_args.args[0].get_header("User-agent"), "Citadel-Release-Healthcheck/1.0")
                else:
                    with self.assertRaisesRegex(ValueError, "health check failed"):
                        demo.verify_health("https://demo.example.com")

    def test_runner_reports_failures_and_checks_source_before_ssh(self):
        value = dev_record(10)
        metadata_file = self.directory / "version.json"
        metadata_file.write_text(json.dumps(value["metadata"]))
        summary = self.directory / "summary.md"
        env = dict(CITADEL_DEMO_HOST="demo.example.com", CITADEL_DEMO_USER="citadel",
                   CITADEL_DEMO_DIRECTORY="/opt/citadel", CITADEL_DEMO_URL="https://demo.example.com",
                   GITHUB_EVENT_NAME="push", GITHUB_REF="refs/tags/" + value["tag"],
                   GITHUB_SHA=value["metadata"]["sourceRevision"], GITHUB_STEP_SUMMARY=str(summary))
        argv = ["demo.py", "--channel", "dev", "--apply", "--repository", "Citadel-P/Citadel",
                "--metadata", str(metadata_file), "--identity", "/unused", "--known-hosts", "/unused"]
        with patch.dict(demo.os.environ, env, clear=True), patch.object(demo.sys, "argv", argv), \
             patch.object(demo, "deployment_payload", return_value=self.payload) as payload, \
             patch.object(demo.subprocess, "check_output", return_value="wrong") as head, \
             patch.object(demo.subprocess, "run") as run:
            with self.assertRaisesRegex(ValueError, "checkout"):
                demo.main()
            run.assert_not_called()
            self.assertIn("failed-preflight", summary.read_text())
            head.return_value = value["metadata"]["sourceRevision"]
            run.side_effect = subprocess.CalledProcessError(1, ["ssh"])
            with self.assertRaises(subprocess.CalledProcessError):
                demo.main()
            self.assertIn("failed-remote-deployment", summary.read_text())
            run.side_effect = None
            run.return_value = subprocess.CompletedProcess([], 0, json.dumps({"status": "deployed", **{k: self.payload[k] for k in remote.IDENTITY}}))
            with patch.object(demo, "verify_health", side_effect=ValueError("health check failed")), self.assertRaises(ValueError):
                demo.main()
            self.assertIn("failed-public-health", summary.read_text())
            self.assertIn(self.payload["core"], summary.read_text())
            self.assertNotIn("private", summary.read_text())
            with patch.object(demo, "verify_health") as health, patch("builtins.print"):
                demo.main()
                health.assert_called_once_with(env["CITADEL_DEMO_URL"])
                run.reset_mock()
                payload.return_value = {"status": "skipped-stale-alias", **self.payload}
                demo.main()
                run.assert_not_called()
                self.assertIn("skipped-stale-alias", summary.read_text())
            payload.return_value = self.payload
            with patch.object(demo, "verify_health"), patch("builtins.print"):
                demo.main()
            ssh = run.call_args.args[0]
            for option in ("BatchMode=yes", "IdentitiesOnly=yes", "StrictHostKeyChecking=yes"):
                self.assertIn(option, ssh)

    def test_failed_https_health_check_fails_job(self):
        with patch.object(demo.urllib.request, "urlopen", side_effect=OSError("unreachable")), \
             patch.object(demo.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "health check failed"):
                demo.verify_health("https://demo.example.com")


class DeploymentWorkflowTests(unittest.TestCase):
    def test_publication_dependencies_channel_gates_and_shared_lock(self):
        source = (ROOT / ".github/workflows/ci.yml").read_text()
        jobs = dict(re.findall(r"^  ([a-z][a-z0-9-]*):\n(.*?)(?=^  [a-z][a-z0-9-]*:|\Z)", source, re.M | re.S))
        for job in ("publish", "publish-development", "deploy-preview", "deploy-demo-stable"):
            with self.subTest(job=job):
                self.assertIn("group: citadel-product-publication", jobs[job])
                self.assertIn("cancel-in-progress: false", jobs[job])
                self.assertIn("queue: max", jobs[job])
        for job, channel, output, dependency, environment, toggle in (
            ("deploy-preview", "dev", "development", "publish-development", "preview", "CITADEL_PREVIEW_DEPLOY_ENABLED"),
            ("deploy-demo-stable", "latest", "release", "publish", "demo", "CITADEL_DEMO_DEPLOY_ENABLED"),
        ):
            block = jobs[job]
            self.assertIn(f"needs: [version, {dependency}]", block)
            condition = re.search(r"^    if: (.+)$", block, re.M).group(1)
            self.assertEqual(condition, f"needs.version.outputs.{output} == 'true' && vars.{toggle} == 'true'")
            self.assertIn(f"environment:\n      name: {environment}\n", block)
            self.assertNotIn("CITADEL_DEMO_CHANNEL", block)
            self.assertIn("ref: ${{ needs.version.outputs.revision }}", block)
            self.assertIn("uses: ./.github/actions/deploy-demo", block)
            self.assertIn(f"channel: {channel}", block)
            self.assertNotIn("ssh ", block)
        action = (ROOT / ".github/actions/deploy-demo/action.yml").read_text()
        self.assertIn('--channel "$CHANNEL"', action)
        self.assertIn("if: always()", action)
        self.assertIn("rm -rf \"$RUNNER_TEMP/demo-ssh\"", action)


if __name__ == "__main__":
    unittest.main()
