"""Documentation consistency checks, not runtime or legal compliance tests."""
import json
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
POLICY = pathlib.Path("docs/content/docs/overview/community-commitment.md")


class CommunityCommitmentTests(unittest.TestCase):
    def setUp(self):
        self.policy = (ROOT / POLICY).read_text(encoding="utf-8")
        self.normalized = " ".join(self.policy.split())

    def test_policy_is_in_product_navigation(self):
        metadata = json.loads((ROOT / POLICY.parent / "meta.json").read_text(encoding="utf-8"))
        self.assertEqual(metadata["pages"].count(POLICY.stem), 1)
        self.assertTrue(self.policy.startswith('---\ntitle: "Community commitment"\n'))

    def test_policy_is_distinct_from_the_source_license(self):
        self.assertIn("Elastic-2.0", self.policy)
        self.assertIn("not OSI-approved open source", self.normalized)
        self.assertIn("does not amend or replace the source license", self.normalized)
        self.assertIn("rights already granted for earlier copies", self.normalized)

    def test_baseline_and_change_notice_are_explicit(self):
        self.assertIn("We will not move established Community capabilities behind a paid product key", self.normalized)
        self.assertIn("0e65fbe5d49d1f66cb12920a56e8b36375c93167", self.policy)
        self.assertIn("dated public notice before the affected release", self.normalized)
        self.assertIn("Paid-license expiry must not delete configuration", self.normalized)

    def test_proposed_free_automation_is_not_described_as_shipped(self):
        self.assertIn("Scheduled backups and webhook-triggered deployments still require Team.", self.normalized)
        review = (ROOT / "docs/COMMUNITY-PACKAGING-REVIEW.md").read_text(encoding="utf-8")
        self.assertIn("not an approved entitlement change", review)
        self.assertIn("Do not advertise proposed free features", review)

    def test_public_entrypoints_and_review_process_link_to_policy(self):
        for path in ("README.md", "COMMERCIAL-LICENSING.md", "docs/LEGAL-RELEASE-CHECKLIST.md", ".github/pull_request_template.md"):
            with self.subTest(path=path):
                text = (ROOT / path).read_text(encoding="utf-8")
                self.assertIn("community-commitment.md", text)


if __name__ == "__main__":
    unittest.main()
