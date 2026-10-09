"""Baseline ELv2 source-tree checks, not a legal compliance certification."""
import hashlib
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
# Unmodified Elastic-maintained ELv2 text (licenses/ELASTIC-LICENSE-2.0.txt).
CANONICAL_LICENSE_BLOB = "809108b857ffd2c2a93cafc5b69e496f3b6ace04"


class LicensingDistributionChecks(unittest.TestCase):
    def read(self, path):
        return (ROOT / path).read_text(encoding="utf-8")

    def test_canonical_elastic_license_is_unmodified(self):
        content = self.read("LICENSE").encode("utf-8")
        blob = b"blob " + str(len(content)).encode("ascii") + b"\0" + content
        self.assertEqual(
            hashlib.sha1(blob, usedforsecurity=False).hexdigest(),
            CANONICAL_LICENSE_BLOB,
            "Keep root LICENSE identical to the canonical ELv2 text.",
        )

    def test_both_images_ship_license_and_known_data_notices(self):
        for filename in ("Dockerfile", "Dockerfile.agent"):
            with self.subTest(filename=filename):
                text = self.read(filename)
                self.assertIn("COPY LICENSE /app/LICENSE", text)
                self.assertIn('org.opencontainers.image.licenses="Elastic-2.0"', text)
                self.assertNotIn("AGPL-3.0", text)
                self.assertIn(
                    "COPY src/features/alerts/LICENSE.unicode "
                    "/app/third-party-notices/Unicode-3.0.txt", text,
                )
                self.assertIn(
                    "COPY src/features/identity/src/authentication/common-passwords.LICENSE "
                    "/app/third-party-notices/Common-Passwords-MIT.txt", text,
                )
        self.assertIn("UNICODE LICENSE V3", self.read("src/features/alerts/LICENSE.unicode"))
        self.assertIn(
            "MIT License", self.read("src/features/identity/src/authentication/common-passwords.LICENSE"),
        )

    def test_ui_notices_match_the_source_license(self):
        for path in (
            "src/frontend/src/features/auth/auth-shell.tsx",
            "src/frontend/src/layout/header.tsx",
        ):
            with self.subTest(path=path):
                text = self.read(path)
                self.assertIn("sourceAndLicenseLinks", text)
                self.assertIn("Source code", text)
                self.assertIn("Elastic License 2.0", text)
                self.assertIn("source-available", text.lower())
                self.assertRegex(text.lower(), r"(?:no|without) warranty")
                self.assertNotIn("AGPL-3.0", text)
        helper = self.read("src/frontend/src/lib/source-license.ts")
        self.assertIn("VITE_CITADEL_BUILD_SHA", helper)
        self.assertIn("VITE_CITADEL_SOURCE_REPOSITORY_URL", helper)

    def test_current_policy_documents_describe_elv2(self):
        for path in (
            "README.md", "COMMERCIAL-LICENSING.md", "CONTRIBUTING.md",
            "TRADEMARKS.md", "docs/LEGAL-RELEASE-CHECKLIST.md",
            "docs/content/docs/overview/licensing.md",
        ):
            with self.subTest(path=path):
                text = self.read(path)
                self.assertIn("Elastic License 2.0", text)
                self.assertNotIn("Citadel is open source", text)
        terms = self.read("COMMERCIAL-LICENSING.md")
        self.assertIn("internal business production", terms)
        self.assertIn("executed written agreement", terms)
        self.assertIn("Team key alone is not a hosting/reseller grant", terms)

    def test_prior_public_grants_remain_a_review_item(self):
        checklist = self.read("docs/LEGAL-RELEASE-CHECKLIST.md")
        self.assertIn("Earlier public licensing snapshots", checklist)
        self.assertIn("ae38ab491c9ddd936936d8b1631791d8e13bf0f9", checklist)
        self.assertIn("does not revoke rights already granted", checklist)


if __name__ == "__main__":
    unittest.main()
