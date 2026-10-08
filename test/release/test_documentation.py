"""Shared Pages publication and mixed-channel ordering regressions."""
import json
import re
import unittest
from unittest.mock import patch

from test_development import dev_record
import test_release
from test_release import FakeBackend, ROOT, release


class DocumentationTests(unittest.TestCase):
    def test_semver_order_across_channels_and_numeric_components(self):
        versions = ["1.2.3-dev.9", "1.2.3-dev.10", "1.2.3", "1.2.4-dev.1", "1.9.9", "1.10.0-dev.1", "1.10.0"]
        self.assertEqual(sorted(reversed(versions), key=release.documentation_order), versions)

    def test_both_ghcr_components_prevent_older_retries(self):
        backend = FakeBackend()
        data = dev_record(9)["metadata"]
        self.assertTrue(release.documentation_eligible(data, backend))
        for component in ("core", "agent"):
            repo = release.POLICY["registries"][component][0]
            for newer in ("1.2.3-dev.10", "1.2.3", "1.2.4-dev.1"):
                with self.subTest(component=component, newer=newer):
                    backend.images = {(repo, newer): "sha256:" + "a" * 64}
                    self.assertFalse(release.documentation_eligible(data, backend))
        backend.images = {(repo, "1.2.3-dev.8"): "sha256:" + "a" * 64}
        self.assertTrue(release.documentation_eligible(data, backend))

    def test_both_publication_jobs_use_pages_before_finalization(self):
        source = (ROOT / ".github/workflows/ci.yml").read_text()
        jobs = dict(re.findall(r"^  ([a-z][a-z0-9-]*):\n(.*?)(?=^  [a-z][a-z0-9-]*:|\Z)", source, re.M | re.S))
        for job in ("publish", "publish-development"):
            block = jobs[job]
            self.assertIn("name: github-pages", block)
            self.assertIn("pages: write", block)
            self.assertIn("id-token: write", block)
            self.assertIn("group: citadel-product-publication", block)
            self.assertIn("DOCS_SITE_URL: ${{ vars.DOCS_SITE_URL }}", block)
            self.assertLess(block.index("release.py promote"), block.index("uses: ./.github/actions/publish-docs"))
            self.assertLess(block.index("uses: ./.github/actions/publish-docs"), block.index("release.py finalize"))
            self.assertIn("CITADEL_DOCS_RESULT: ${{ steps.documentation.outputs.result }}", block)
        action = (ROOT / ".github/actions/publish-docs/action.yml").read_text()
        self.assertIn("uses: actions/deploy-pages@v4", action)
        self.assertIn("if: steps.order.outputs.eligible == 'true'", action)


class DevelopmentDocumentationBoundaryTests(unittest.TestCase):
    main = test_release.CommandBoundaryTests.main

    def setUp(self):
        test_release.CommandBoundaryTests.setUp(self)
        self.value = dev_record()
        self.value["status"] = "images-verified"
        self.file.write_text(json.dumps(self.value["metadata"]))
        self.backend.records = {self.value["tag"]: self.value}
        self.env.update(GITHUB_EVENT_NAME="push", GITHUB_REF="refs/tags/" + self.value["tag"], GITHUB_SHA="a" * 40)

    def test_development_docs_are_eligible_after_verification(self):
        with patch.dict(release.os.environ, self.env, clear=True), patch("builtins.print") as output:
            self.main("docs")
            self.assertEqual(output.call_args.args, ("eligible=true",))
        self.assertFalse(self.backend.operations)

    def test_missing_or_wrong_docs_url_blocks_finalization(self):
        self.backend.apply = True
        for result in ("", "https://wrong.example.invalid", "skipped-older-release"):
            with self.subTest(result=result), patch.dict(release.os.environ, dict(self.env, CITADEL_DOCS_RESULT=result), clear=True), \
                 patch.object(release.version, "git", side_effect=["tag", "a" * 40]):
                with self.assertRaisesRegex(ValueError, "successful documentation"):
                    self.main("finalize", apply=True)
        self.assertFalse(self.backend.operations)

    def test_missing_docs_url_fails_preflight(self):
        for url in ("", "http://docs.example.invalid"):
            with self.subTest(url=url):
                with self.assertRaises(ValueError):
                    release.preflight(self.value["metadata"], self.backend, dict(self.env, DOCS_SITE_URL=url))

    def test_unverified_images_cannot_publish_docs(self):
        self.value["status"] = "prepared"
        with patch.dict(release.os.environ, self.env, clear=True):
            with self.assertRaisesRegex(ValueError, "verified exact images"):
                self.main("docs")

    def test_older_development_finalizes_only_with_skip_result(self):
        self.backend.apply = True
        self.backend.images["ghcr.io/citadel-p/citadel", "1.2.3-dev.2"] = "sha256:" + "a" * 64
        with patch.dict(release.os.environ, dict(self.env, CITADEL_DOCS_RESULT="skipped-older-release"), clear=True), \
             patch.object(release.version, "git", side_effect=["tag", "a" * 40]), \
             patch.object(self.backend, "command", create=True):
            self.main("finalize", apply=True)
        self.assertEqual(self.backend.load(self.value["tag"])["steps"]["documentation"], "skipped-older-release")
