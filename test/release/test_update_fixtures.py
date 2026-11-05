"""Keep the Rust updater's wire fixtures aligned with the actual publisher."""
import json
import copy
from pathlib import Path
import tempfile
import unittest

from test_release import release


class UpdateManifestFixtures(unittest.TestCase):
    def test_fixtures_are_exact_publisher_output(self):
        for path in (Path(__file__).parent / 'fixtures').glob('update-*.json'):
            with self.subTest(path=path), tempfile.TemporaryDirectory() as temporary:
                fixture = json.loads(path.read_text())
                record = dict(tag=fixture['tag'], metadata=fixture['metadata'],
                              signingIdentity=fixture['signatures']['certificateIdentity'],
                              artifacts={'release-notes.md': fixture['releaseNotesSha256']},
                              indexes={c: images[0]['digest'].split('@')[1]
                                       for c, images in fixture['images'].items()},
                              steps={'signed:' + ref: digest for ref, digest in fixture['verifiedSignatures'].items()})
                record['evidence'] = {arch: {'upgradeDescriptor': fixture['upgradeDescriptor']}
                                      for arch in release.POLICY['architectures']}
                release.public_files(record, Path(temporary))
                self.assertEqual(path.read_bytes(), (Path(temporary) / 'release-manifest.json').read_bytes())

    def test_publication_retains_accepted_descriptor_and_rejects_disagreement(self):
        fixture = json.loads((Path(__file__).parent / 'fixtures/update-stable.json').read_text())
        record = dict(tag=fixture['tag'], metadata=fixture['metadata'],
                      signingIdentity=fixture['signatures']['certificateIdentity'],
                      artifacts={'release-notes.md': fixture['releaseNotesSha256']},
                      indexes={c: images[0]['digest'].split('@')[1] for c, images in fixture['images'].items()},
                      steps={'signed:' + ref: digest for ref, digest in fixture['verifiedSignatures'].items()},
                      evidence={arch: {'upgradeDescriptor': copy.deepcopy(fixture['upgradeDescriptor'])}
                                for arch in release.POLICY['architectures']})
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            release.public_files(record, directory)
            actual = json.loads((directory / 'release-manifest.json').read_text())
            # Synthetic accepted catalog differs from this checkout. Recovery
            # must retain it, not quietly replace it with the current catalog.
            self.assertEqual(actual['upgradeDescriptor'], fixture['upgradeDescriptor'])
            record['evidence']['arm64']['upgradeDescriptor']['targetCatalogSha256'] = 'e'*64
            with self.assertRaisesRegex(ValueError, 'descriptors differ'):
                release.public_files(record, directory)
            del record['evidence']['arm64']
            with self.assertRaisesRegex(ValueError, 'descriptors differ'):
                release.public_files(record, directory)
            # Original retained publications without this new evidence are
            # recovered without rewriting their immutable public attachment.
            record.pop('evidence')
            release.public_files(record, directory)
            self.assertNotIn('upgradeDescriptor', json.loads((directory / 'release-manifest.json').read_text()))
