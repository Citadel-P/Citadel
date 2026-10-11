"""Real Git histories exercise the pinned changelog engine without remote writes."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from test_release import ROOT

spec = importlib.util.spec_from_file_location('notes', ROOT / 'src/tools/release/notes.py')
notes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notes)
BINARY = Path(os.environ.get('CITADEL_GIT_CLIFF', Path.home() / '.cache/citadel-tools' / ('git-cliff-' + notes.PIN['version']) / 'git-cliff'))


@unittest.skipUnless(BINARY.is_file(), 'install the pinned tool: python3 src/tools/release/notes.py install')
class NotesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git('init', '-q', '-b', 'main')
        self.git('config', 'user.name', 'Fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.commit('feat: initial feature')
        self.git('tag', 'v1.0.0')
        self.output = self.root / 'output'

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True, stderr=subprocess.PIPE).strip()

    def commit(self, message):
        self.git('commit', '--allow-empty', '-qm', message)
        return self.git('rev-parse', 'HEAD')

    def generate(self, tag='v1.1.0'):
        self.git('tag', tag)
        metadata = dict(displayVersion=tag[1:], sourceRevision=self.git('rev-parse', 'HEAD'), releaseEligible='-dev.' not in tag)
        self.commit('feat: future change must not leak')
        report = notes.generate(self.root, metadata, self.output, BINARY, 'https://docs.example.invalid')
        return (self.output / 'release-notes.md').read_text(), report

    def test_stable_range_includes_development_tags_and_excludes_later_head(self):
        self.commit('feat: before development tag'); self.git('tag', 'v1.1.0-dev.1')
        self.commit('fix: after development tag')
        self.commit('perf: reduce queries')
        self.commit('refactor: simpler configuration')
        for noise in ['ci: workflow cleanup', 'chore(deps): upgrade library', 'docs: typo', 'test: fixtures']:
            self.commit(noise)
        text, report = self.generate()
        self.assertEqual(report['previousTag'], 'v1.0.0')
        for title in ['Features', 'Fixes', 'Performance', 'Improvements', 'Before development tag', 'After development tag']:
            self.assertIn(title, text)
        for noise in ['future change', 'workflow cleanup', 'upgrade library', 'typo', 'fixtures', 'Initial feature']:
            self.assertNotIn(noise, text)
        self.assertFalse(report['review'])
        self.assertIn('docker.io/citadelplane/citadel:1.1.0', text)

    def test_direct_squash_and_merge_commits_with_review_and_breaking_noise(self):
        self.commit('fix: repair squash flow (#12)')
        self.git('checkout', '-qb', 'feature')
        self.commit('feat: add new view')
        self.git('checkout', '-q', 'main')
        self.git('merge', '--no-ff', 'feature', '-m', 'Merge pull request #13 from owner/feature\n\nfeat: add new view')
        unclear = self.commit('miscellaneous adjustments')
        self.commit('chore!: remove old configuration\n\nBREAKING CHANGE: Rename OLD_URL to NEW_URL.')
        self.commit('removed obsolete API\n\nBREAKING CHANGE: Clients must use v2.')
        text, report = self.generate()
        self.assertEqual(text.count('Add new view'), 1)
        self.assertIn('Repair squash flow (#12)', text)
        self.assertIn('Rename OLD_URL to NEW_URL.', text)
        self.assertIn('Clients must use v2.', text)
        self.assertIn('Breaking changes', text)
        self.assertIn(unclear, [item['commit'] for item in report['review']])
        self.assertNotIn('Merge pull request', text)

    def test_development_previous_dev_and_first_release(self):
        self.commit('feat: earlier dev'); self.git('tag', 'v1.1.0-dev.1')
        self.commit('fix: current dev')
        text, report = self.generate('v1.1.0-dev.2')
        self.assertEqual(report['previousTag'], 'v1.1.0-dev.1')
        self.assertNotIn('Earlier dev', text)
        self.assertNotIn('docker.io/', text)
        self.assertIsNone(notes.predecessor(self.root, 'v1.0.0', self.git('rev-parse', 'v1.0.0')))

    def test_tag_identity_mismatch_is_rejected(self):
        self.git('tag', 'v1.1.0')
        revision = self.commit('feat: later')
        with self.assertRaisesRegex(ValueError, 'exact tagged revision'):
            notes.generate(self.root, dict(displayVersion='1.1.0', sourceRevision=revision), self.output, BINARY, 'https://docs.example.invalid')
