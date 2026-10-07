"""Baseline source-tree checks, not a legal compliance certification."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]


class LicensingDistributionChecks(unittest.TestCase):
    def test_source_license_and_commercial_distinction(self):
        text = (ROOT / "LICENSE").read_text(encoding="utf-8")
        self.assertTrue(text.startswith("GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3, 19 November 2007"))
        self.assertIn("13. Remote Network Interaction; Use with the GNU General Public License.", text)
        commercial = (ROOT / "COMMERCIAL-LICENSING.md").read_text(encoding="utf-8")
        self.assertIn("executed written agreement", commercial)

    def test_both_images_ship_license_and_known_data_notices(self):
        for filename in ("Dockerfile", "Dockerfile.agent"):
            with self.subTest(filename=filename):
                text = (ROOT / filename).read_text(encoding="utf-8")
                self.assertIn("COPY LICENSE /app/LICENSE", text)
                self.assertIn('org.opencontainers.image.licenses="AGPL-3.0-only"', text)
                self.assertIn("COPY src/features/alerts/LICENSE.unicode /app/third-party-notices/Unicode-3.0.txt", text)
                self.assertIn("COPY src/features/identity/src/authentication/common-passwords.LICENSE /app/third-party-notices/Common-Passwords-MIT.txt", text)
        self.assertIn("UNICODE LICENSE V3", (ROOT / "src/features/alerts/LICENSE.unicode").read_text(encoding="utf-8"))
        self.assertIn("MIT License", (ROOT / "src/features/identity/src/authentication/common-passwords.LICENSE").read_text(encoding="utf-8"))

    def test_source_offer_on_auth_and_account_screens(self):
        unauth = (ROOT / "src/frontend/src/features/auth/auth-shell.tsx").read_text(encoding="utf-8")
        auth = (ROOT / "src/frontend/src/layout/header.tsx").read_text(encoding="utf-8")
        helper = (ROOT / "src/frontend/src/lib/source-license.ts").read_text(encoding="utf-8")
        for text in (unauth, auth):
            self.assertIn("sourceAndLicenseLinks", text)
            self.assertIn("Source code", text)
            self.assertRegex(text.lower(), r"(?:no|without) warranty")
        self.assertIn("License terms", unauth)
        self.assertIn("AGPL-3.0 license", auth)
        self.assertIn("VITE_CITADEL_BUILD_SHA", helper)
        self.assertIn("VITE_CITADEL_SOURCE_REPOSITORY_URL", helper)


if __name__ == "__main__":
    unittest.main()
