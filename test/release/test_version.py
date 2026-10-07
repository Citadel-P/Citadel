import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("version", ROOT / "src/tools/build/version.py")
version = importlib.util.module_from_spec(spec)
spec.loader.exec_module(version)


class VersionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repository"
        self.root.mkdir()
        version.run("git", "init", "-q", "-b", "main", self.root)
        self.git("config", "user.name", "Release fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.config = json.loads((ROOT / "version.json").read_text())
        self.config["version"] = "1.2.3"
        self.write_version()
        (self.root / ".gitignore").write_text("/output/\n")
        self.commit()

    def git(self, *args):
        return version.git(self.root, *args)

    def write_version(self):
        (self.root / "version.json").write_text(json.dumps(self.config))

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")
        self.git("update-ref", "refs/remotes/origin/main", "HEAD")

    def resolve(self, **kwargs):
        return version.resolve(self.root, **kwargs)

    def test_context_and_repeatability(self):
        self.git("tag", "-a", "v1.2.3", "-m", "fixture")
        release = self.resolve(event="push", ref="refs/tags/v1.2.3")
        self.assertTrue(release["releaseEligible"])
        self.assertTrue(release["nbgvPublicRelease"])
        self.assertEqual(release["displayVersion"], "1.2.3")
        self.assertEqual(release, self.resolve(event="push", ref="refs/tags/v1.2.3"))
        self.assertEqual(release["sourceRevision"], self.git("rev-parse", "HEAD"))
        for event, ref in [("local", ""), ("push", "refs/heads/main"), ("push", "refs/heads/feature"),
                           ("pull_request", "refs/pull/1/merge"), ("workflow_dispatch", "refs/tags/v1.2.3")]:
            data = self.resolve(event=event, ref=ref)
            self.assertFalse(data["releaseEligible"])
            self.assertTrue(data["nbgvPublicRelease"])
            self.assertRegex(data["displayVersion"], r"^1\.2\.3-dev\.\d+\.g[a-f0-9]{12}$")

    def test_dirty_tracked_untracked_and_ignored(self):
        (self.root / "output").mkdir()
        (self.root / "output/build").write_text("ignored")
        self.assertFalse(self.resolve()["dirty"])
        (self.root / "new-file").write_text("untracked")
        self.assertTrue(self.resolve()["displayVersion"].endswith(".dirty"))
        self.assertNotIn("dirty", self.resolve()["sourceRevision"])
        self.commit()
        (self.root / "new-file").write_text("tracked edit")
        self.assertTrue(self.resolve()["dirty"])

    def test_rejects_invalid_release_inputs(self):
        for tag in ["v01.2.3", "v1.2.4", "v1.2.3-rc.1", "v1.2", "v1.2.3.4",
                    "v1.2.3-dev.0", "v1.2.3-dev.01", "v1.2.3-dev-1", "v1.2.4-dev.1"]:
            self.git("tag", "-a", tag, "-m", "fixture")
            with self.assertRaises(ValueError):
                self.resolve(event="push", ref="refs/tags/" + tag)
        self.git("tag", "v1.2.3")
        with self.assertRaisesRegex(ValueError, "annotated"):
            self.resolve(event="push", ref="refs/tags/v1.2.3")
        self.git("tag", "-d", "v1.2.3")
        self.git("tag", "-a", "v1.2.3", "-m", "fixture")
        (self.root / "dirty").touch()
        with self.assertRaisesRegex(ValueError, "clean"):
            self.resolve(event="push", ref="refs/tags/v1.2.3")
        (self.root / "dirty").unlink()
        with self.assertRaisesRegex(ValueError, "expected full"):
            self.resolve(expected_sha="a" * 40)

    def test_numbered_development_tag_uses_committed_base(self):
        version.run("python3", ROOT / "src/tools/build/version.py", "tag", "--dev", "12", "--root", self.root)
        ref = "refs/tags/v1.2.3-dev.12"
        data = self.resolve(event="push", ref=ref)
        self.assertEqual(data["productVersion"], "1.2.3")
        self.assertEqual(data["displayVersion"], "1.2.3-dev.12")
        self.assertTrue(data["nbgvPublicRelease"])
        self.assertTrue(data["publicationEligible"])
        self.assertTrue(data["developmentReleaseEligible"])
        self.assertFalse(data["releaseEligible"])
        self.assertTrue(data["informationalVersion"].startswith("1.2.3-dev.12+height."))
        for event in ["local", "workflow_dispatch", "pull_request"]:
            self.assertFalse(self.resolve(event=event, ref=ref)["publicationEligible"])
        with self.assertRaises(RuntimeError):
            version.run("python3", ROOT / "src/tools/build/version.py", "tag", "--dev", "12", "--root", self.root)
        (self.root / "dirty").touch()
        with self.assertRaisesRegex(ValueError, "clean"):
            self.resolve(event="push", ref=ref)
        (self.root / "dirty").unlink()
        self.git("tag", "v1.2.3-dev.13")
        with self.assertRaisesRegex(ValueError, "annotated"):
            self.resolve(event="push", ref="refs/tags/v1.2.3-dev.13")

    def test_ci_fetch_restores_annotated_tag_after_checkout_peels_it(self):
        ref = "refs/tags/v1.2.3-dev.1"
        self.git("tag", "-a", ref.removeprefix("refs/tags/"), "-m", "fixture")
        tag_object = self.git("rev-parse", ref)
        source = self.git("rev-parse", "HEAD")
        checkout = Path(self.temp.name) / "checkout"
        version.run("git", "clone", self.root.as_uri(), checkout)
        version.git(checkout, "checkout", "--detach", source)
        # actions/checkout can fetch the event SHA into the tag ref, replacing
        # the annotated tag object with its commit in the disposable checkout.
        version.git(checkout, "update-ref", ref, source)
        self.assertEqual(version.git(checkout, "cat-file", "-t", ref), "commit")
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        fetch = next(line.strip() for line in workflow.splitlines()
                     if line.strip().startswith("git fetch origin "))
        subprocess.run(["bash", "-c", fetch], cwd=checkout, check=True,
                       capture_output=True, text=True)
        self.assertEqual(version.git(checkout, "rev-parse", ref), tag_object)
        self.assertEqual(version.git(checkout, "rev-parse", "HEAD"), source)
        result = version.resolve(checkout, event="push", ref=ref, expected_sha=source)
        self.assertTrue(result["developmentReleaseEligible"])
        self.assertEqual(result["displayVersion"], "1.2.3-dev.1")

    def test_manual_patch_height_and_uncommitted_version(self):
        first = self.resolve()
        (self.root / "code").write_text("change")
        self.commit()
        next_build = self.resolve()
        self.assertEqual(next_build["productVersion"], first["productVersion"])
        self.assertGreater(next_build["versionHeight"], first["versionHeight"])
        self.config["version"] = "1.2.4"
        self.write_version()
        with self.assertRaisesRegex(ValueError, "Commit version.json"):
            self.resolve()
        self.commit()
        self.assertEqual(self.resolve()["productVersion"], "1.2.4")

    def test_uncommitted_configuration_allows_development_but_not_release(self):
        self.git("tag", "-a", "v1.2.3", "-m", "fixture")
        self.config["gitCommitIdShortFixedLength"] = 10
        self.write_version()
        for staged in [False, True]:
            with self.subTest(staged=staged):
                if staged:
                    self.git("add", "version.json")
                data = self.resolve()
                self.assertEqual(data["productVersion"], "1.2.3")
                self.assertTrue(data["dirty"])
                self.assertFalse(data["releaseEligible"])
                self.assertRegex(data["displayVersion"], r"^1\.2\.3-dev\.\d+\.g[a-f0-9]{12}\.dirty$")
                with self.assertRaisesRegex(ValueError, "clean"):
                    self.resolve(event="push", ref="refs/tags/v1.2.3")
        output = version.run("python3", ROOT / "src/tools/build/version.py", "exec", "--root", self.root,
                             "--", "python3", "-c", "import os; print(os.environ['CITADEL_VERSION'])")
        self.assertEqual(output, data["displayVersion"])

    def test_shallow_archive_and_history(self):
        shallow = Path(self.temp.name) / "shallow"
        version.run("git", "clone", "--depth=1", self.root.as_uri(), shallow)
        with self.assertRaisesRegex(ValueError, "Shallow"):
            version.resolve(shallow)
        archive = Path(self.temp.name) / "archive"
        archive.mkdir()
        shutil.copy(self.root / "version.json", archive)
        data = version.resolve(archive, allow_fallback=True)
        self.assertEqual(data["displayVersion"], "1.2.3-dev.unknown")
        self.assertIsNone(data["sourceRevision"])
        with self.assertRaisesRegex(ValueError, "cannot use fallback"):
            version.resolve(archive, event="push", ref="refs/tags/v1.2.3", allow_fallback=True)
        self.git("checkout", "-qb", "maintenance")
        (self.root / "change").touch()
        self.git("add", ".")
        self.git("commit", "-qm", "not on main")
        self.git("tag", "-a", "v1.2.3", "-m", "fixture")
        with self.assertRaises(RuntimeError):
            self.resolve(event="push", ref="refs/tags/v1.2.3")

    def test_annotated_tag_command_refuses_repointing(self):
        version.run("python3", ROOT / "src/tools/build/version.py", "tag", "--root", self.root)
        self.assertEqual(self.git("cat-file", "-t", "refs/tags/v1.2.3"), "tag")
        with self.assertRaises(RuntimeError):
            version.run("python3", ROOT / "src/tools/build/version.py", "tag", "--root", self.root)


class EmbedTests(unittest.TestCase):
    def test_same_checkout_rebuilds_on_metadata_changes_and_fallback(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            crate = root / "src/probe"
            crate.mkdir(parents=True)
            (root / "version.json").write_text('{"version":"1.2.3"}')
            (crate / "Cargo.toml").write_text('[package]\nname="version-probe"\nversion="0.0.0"\nedition="2024"\n[build-dependencies]\nserde_json="1"\n')
            shutil.copy(ROOT / "src/tools/build/version.rs", crate / "build.rs")
            (crate / "src").mkdir()
            (crate / "src/main.rs").write_text('fn main(){println!("{}|{}",env!("CITADEL_BUILD_VERSION"),env!("CITADEL_BUILD_INFORMATIONAL_VERSION"));}')
            env = {k: v for k, v in os.environ.items() if not k.startswith("CITADEL_")}
            for display, info, expected in [
                ("1.2.3-dev.5.gabcdef", "1.2.3-dev.5.gabcdef+height.5.sha.abcdef", "1.2.3-dev.5.gabcdef|1.2.3-dev.5.gabcdef+height.5.sha.abcdef"),
                ("1.2.3", "1.2.3-other+sha.abcdef", "1.2.3|1.2.3-other+sha.abcdef"),
                ("1.2.4", None, "1.2.4|1.2.4"),
                (None, "1.2.5+sha.abcdef", "1.2.5|1.2.5+sha.abcdef"),
                (None, None, "1.2.3-dev.unknown|1.2.3-dev.unknown+source.unknown"),
            ]:
                supplied = dict(env)
                if display: supplied["CITADEL_VERSION"] = display
                if info: supplied["CITADEL_INFORMATIONAL_VERSION"] = info
                self.assertEqual(version.run("cargo", "run", "--quiet", "--manifest-path", crate / "Cargo.toml", env=supplied), expected)


if __name__ == "__main__":
    unittest.main()
