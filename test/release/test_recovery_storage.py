"""Exercise the real journal/retention implementation against an in-memory GitHub."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from test_release import metadata, record, release


class MemoryGitHub(release.Backend):
    def __init__(self):
        super().__init__('Citadel-P/Citadel', apply=True)
        self.items, self.bytes, self.operations = {}, {}, []
        self.fail_upload = None
        self.fail_delete = False

    def releases(self):
        return copy.deepcopy(list(self.items.values()))

    def release(self, tag):
        return copy.deepcopy(self.items.get(tag))

    def asset(self, item, name, path):
        key = item['tag_name'], name
        if key not in self.bytes:
            raise RuntimeError('Missing durable release artifact: ' + name)
        Path(path).write_bytes(self.bytes[key])

    def command(self, *args, write=False):
        if not write:
            raise AssertionError('Unexpected external read: ' + repr(args))
        if not self.apply:
            raise RuntimeError('External mutation blocked: dry run')
        self.operations.append(args)
        if args[:2] == ('gh', 'api') and args[-1] == 'DELETE':
            if self.fail_delete:
                raise RuntimeError('Interrupted cleanup')
            release_id = int(args[2].rsplit('/', 1)[1])
            tag = next(t for t, item in self.items.items() if item['id'] == release_id)
            del self.items[tag]
            self.bytes = {k: v for k, v in self.bytes.items() if k[0] != tag}
        elif args[:2] == ('gh', 'api'):
            data = json.loads(Path(args[-1]).read_text())
            tag = data['tag_name']
            if tag in self.items:
                raise ValueError('Duplicate release')
            self.items[tag] = dict(data, assets=[], id=len(self.items) + 1, prerelease=False)
        elif args[:3] == ('gh', 'release', 'create'):
            tag = args[3]
            self.items[tag] = dict(tag_name=tag, draft=True, assets=[], id=len(self.items) + 1, prerelease=False)
        elif args[:3] == ('gh', 'release', 'upload'):
            tag, path = args[3], Path(args[4])
            if path.name == self.fail_upload:
                raise RuntimeError('Interrupted upload')
            if (tag, path.name) in self.bytes:
                raise ValueError('Cannot clobber an immutable asset')
            self.bytes[tag, path.name] = path.read_bytes()
            self.items[tag]['assets'].append(dict(name=path.name))
        elif args[:3] == ('gh', 'release', 'edit'):
            self.items[args[3]]['draft'] = False
            self.items[args[3]]['prerelease'] = '--prerelease' in args
        else:
            raise AssertionError('Unexpected mutation: ' + repr(args))
        return '{}'

    def action_artifact(self, record, name, destination):
        raise ValueError('Original Actions artifact expired; do not rebuild this release')


class RecoveryStorageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.backend = MemoryGitHub()
        self.value = record()
        self.data = self.value['metadata']
        self.value['signingIdentity'] = f"https://github.com/{self.backend.repository}/.github/workflows/ci.yml@refs/tags/{self.value['tag']}"
        self.storage = 'citadel-recovery/' + self.value['tag']
        self.env = patch.dict(release.os.environ, {'GITHUB_RUN_ID': '42'}, clear=True)
        self.env.start(); self.addCleanup(self.env.stop)
        self.backend.prepare(self.data)

    def docs(self):
        for name in ['docs.tar', 'api-docs.tar']:
            (self.root / name).write_bytes(b'accepted ' + name.encode())
        (self.root / 'documentation-inputs.json').write_text(json.dumps(dict(metadata=self.data,
            artifacts={name: release.version.sha256(self.root / name) for name in ['docs.tar', 'api-docs.tar']})))

    def journal(self):
        self.value.update(schemaVersion=2, recoveryTag=self.storage, sequence=0)
        self.backend.save(self.value)

    def test_reservation_is_draft_without_creating_git_tag_and_binds_original_run(self):
        self.assertTrue(self.backend.items[self.storage]['draft'])
        self.assertEqual(self.backend.reservation(self.data)['sourceRunId'], '42')
        self.assertIsNone(self.backend.load(self.value['tag']))
        count = len(self.backend.operations)
        self.backend.prepare(self.data)
        self.assertEqual(len(self.backend.operations), count)
        with patch.dict(release.os.environ, GITHUB_RUN_ID='43'):
            with self.assertRaisesRegex(ValueError, 'original run'):
                self.backend.recover_accepted('candidates-amd64', self.root, self.data)
        with self.assertRaisesRegex(ValueError, 'identity differs'):
            self.backend.reservation(dict(self.data, sourceRevision='b' * 40))
        self.assertTrue(all(op[:2] != ('git', 'tag') for op in self.backend.operations))

    def test_reservation_creation_interruption_fails_closed(self):
        backend = MemoryGitHub(); backend.fail_upload = 'reservation.json'
        with self.assertRaisesRegex(RuntimeError, 'Interrupted'):
            backend.prepare(self.data)
        with self.assertRaisesRegex(RuntimeError, 'Missing durable'):
            backend.prepare(self.data)

    def test_accepted_outputs_recover_after_scripts_expiry_in_another_run(self):
        self.docs()
        self.backend.retain_validation('release-documentation', self.root, self.data)
        target = self.root / 'recovered'
        with patch.dict(release.os.environ, GITHUB_RUN_ID='99'):
            self.assertTrue(self.backend.recover_accepted('release-documentation', target, self.data))
        self.assertEqual((target / 'docs.tar').read_bytes(), (self.root / 'docs.tar').read_bytes())
        count = len(self.backend.operations)
        self.backend.retain_validation('release-documentation', self.root, self.data)
        self.assertEqual(count, len(self.backend.operations))

    def test_partial_retention_never_accepts_substitute_even_if_it_passes_new_validation(self):
        self.docs(); self.backend.fail_upload = 'api-docs.tar'
        with self.assertRaisesRegex(RuntimeError, 'Interrupted'):
            self.backend.retain_validation('release-documentation', self.root, self.data)
        with self.assertRaisesRegex(ValueError, 'expired'):
            self.backend.recover_accepted('release-documentation', self.root / 'retry', self.data)
        self.backend.fail_upload = None
        self.backend.retain_validation('release-documentation', self.root, self.data)
        (self.root / 'api-docs.tar').write_bytes(b'rebuilt')
        manifest = self.root / 'documentation-inputs.json'
        data = json.loads(manifest.read_text()); data['artifacts']['api-docs.tar'] = release.version.sha256(self.root / 'api-docs.tar')
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'do not substitute'):
            self.backend.retain_validation('release-documentation', self.root, self.data)

    def test_candidate_archives_recover_only_the_accepted_checksums(self):
        for component in ["agent", "core"]:
            (self.root / f"{component}-amd64.oci.tar").write_bytes(component.encode())
        evidence = dict(metadata=self.data, passed=True, artifacts={
            c: release.version.sha256(self.root / f"{c}-amd64.oci.tar") for c in ["agent", "core"]})
        path = self.root / "evidence-amd64.json"; path.write_text(json.dumps(evidence))
        self.backend.retain_validation("candidates-amd64", self.root, self.data)
        with patch.dict(release.os.environ, GITHUB_RUN_ID="99"):
            self.assertTrue(self.backend.recover_accepted("candidates-amd64", self.root / "retry", self.data))
        self.backend.bytes[self.storage, "core-amd64.oci.tar"] = b"substitute"
        with self.assertRaisesRegex(ValueError, "identity lost"):
            self.backend.recover_accepted("candidates-amd64", self.root / "retry", self.data)

    def test_internal_journal_and_all_retained_inputs_recover_without_actions(self):
        path = self.root / "docs.tar"; path.write_bytes(b"original")
        self.value["artifacts"] = {path.name: release.version.sha256(path)}
        self.value["steps"] = {}
        self.journal()
        release.retain(self.backend, self.value, self.root)
        self.assertTrue(self.value["steps"]["artifacts-retained"])
        target = self.root / "retry"
        recovered = release.recover(self.backend, self.value["tag"], target)
        self.assertEqual(recovered, self.value)
        self.assertEqual((target / "docs.tar").read_bytes(), b"original")
        self.assertNotIn(self.value["tag"], self.backend.items)

    def test_corrupt_or_incomplete_durable_validation_inventory_stops_recovery(self):
        self.docs(); self.backend.retain_validation('release-documentation', self.root, self.data)
        self.backend.bytes[self.storage, 'docs.tar'] = b'tampered'
        with self.assertRaisesRegex(ValueError, 'identity lost'):
            self.backend.recover_accepted('release-documentation', self.root / 'retry', self.data)

    def test_journal_stays_separate_and_rejects_published_recovery_storage(self):
        self.journal()
        self.assertEqual(self.backend.load(self.value['tag']), self.value)
        self.assertNotIn(self.value['tag'], self.backend.items)
        self.backend.items[self.storage]['draft'] = False
        with self.assertRaisesRegex(ValueError, 'remain a draft'):
            self.backend.load(self.value['tag'])

    def test_missing_recovery_store_with_public_release_never_rebuilds(self):
        self.backend.items = {self.value['tag']: dict(tag_name=self.value['tag'], draft=False, assets=[dict(name='release-manifest.json')])}
        with self.assertRaisesRegex(RuntimeError, 'Missing durable release artifact'):
            self.backend.load(self.value['tag'])

    def presentation(self):
        notes = self.root / 'release-notes.md'; notes.write_text('## Fixes\n\n- A useful fix.\n')
        self.value['artifacts'][notes.name] = release.version.sha256(notes)
        for component, repos in release.repositories(self.data).items():
            for repo in repos:
                self.value["steps"]["signed:" + repo + ":" + self.data["displayVersion"]] = self.value["indexes"][component]
        self.journal(); self.backend.upload(self.storage, notes)
        return release.prepare_presentation(self.backend, self.value, self.root)

    def test_public_draft_has_only_manifest_and_checksum_and_verified_digest_references(self):
        self.presentation()
        draft = self.backend.release(self.value['tag'])
        self.assertTrue(draft['draft'])
        self.assertEqual({a['name'] for a in draft['assets']}, {'release-manifest.json', 'SHA256SUMS'})
        public = json.loads(self.backend.bytes[self.value['tag'], 'release-manifest.json'])
        self.assertEqual(public['images']['agent'][0]['digest'], 'ghcr.io/citadel-p/citadel.agent@' + self.value['indexes']['agent'])
        count = len(self.backend.operations)
        release.prepare_presentation(self.backend, self.value, self.root)
        self.assertEqual(len(self.backend.operations), count)

    def test_public_upload_interruption_keeps_draft_and_retry_uses_same_manifest(self):
        self.backend.fail_upload = 'SHA256SUMS'
        with self.assertRaisesRegex(RuntimeError, 'Interrupted'):
            self.presentation()
        self.assertTrue(self.backend.release(self.value['tag'])['draft'])
        original = self.backend.bytes[self.value['tag'], 'release-manifest.json']
        self.backend.fail_upload = None
        release.prepare_presentation(self.backend, self.value, self.root)
        self.assertEqual(original, self.backend.bytes[self.value['tag'], 'release-manifest.json'])
        self.backend.bytes[self.value['tag'], 'release-manifest.json'] = b'changed'
        with self.assertRaisesRegex(ValueError, 'identity differs'):
            release.prepare_presentation(self.backend, self.value, self.root)

    def test_unexpected_public_asset_and_changed_notes_stop_without_deletion(self):
        self.presentation()
        self.backend.items[self.value['tag']]['assets'].append(dict(name='old-internal.tar'))
        with self.assertRaisesRegex(ValueError, 'never auto-delete'):
            release.prepare_presentation(self.backend, self.value, self.root)
        self.backend.bytes[self.storage, 'release-notes.md'] = b'changed'
        with self.assertRaisesRegex(ValueError, 'notes identity differs'):
            release.prepare_presentation(self.backend, self.value, self.root)

    def test_legacy_release_journal_remains_readable_in_place(self):
        self.backend.items.pop(self.storage)
        self.backend.save(self.value)
        # Real gh create uploads the supplied journal in the same command; our
        # fake leaves that part explicit for this legacy-only fixture.
        self.backend.items[self.value['tag']]['assets'] = [dict(name='release-record-000001.json')]
        self.backend.bytes[self.value['tag'], 'release-record-000001.json'] = json.dumps(self.value).encode()
        self.assertEqual(self.backend.load(self.value['tag']), self.value)

    def test_finalization_exposes_only_the_product_after_public_assets_are_verified(self):
        self.finalize_and_check_cleanup()

    def test_development_finalization_cleans_only_its_own_recovery_draft(self):
        from test_development import dev_record
        self.value = dev_record()
        self.data = self.value['metadata']
        self.storage = 'citadel-recovery/' + self.value['tag']
        self.value['signingIdentity'] = f"https://github.com/{self.backend.repository}/.github/workflows/ci.yml@refs/tags/{self.value['tag']}"
        self.backend.prepare(self.data)
        self.finalize_and_check_cleanup()
        self.assertTrue(self.backend.release(self.value['tag'])['prerelease'])
        self.assertIn('citadel-recovery/v1.2.3', self.backend.items)

    def finalize_and_check_cleanup(self):
        self.presentation()
        self.value["status"] = "images-verified"; self.backend.save(self.value)
        path = self.root / "version.json"; path.write_text(json.dumps(self.data))
        env = dict(GITHUB_EVENT_NAME="push", GITHUB_REF="refs/tags/" + self.value["tag"],
                   GITHUB_SHA=self.data["sourceRevision"], DOCS_SITE_URL="https://docs.example.invalid",
                   CITADEL_DOCS_RESULT="https://docs.example.invalid")
        with patch.object(release, "Backend", return_value=self.backend), \
             patch.object(release, "preflight", return_value=None), \
             patch.object(release, "documentation_eligible", return_value=True), \
             patch.object(release.version, "git", side_effect=["tag", self.data["sourceRevision"]]), \
             patch.dict(release.os.environ, env), \
             patch.object(release.sys, "argv", ["release.py", "finalize", "--apply", "--repository", self.backend.repository,
                "--metadata", str(path), "--directory", str(self.root)]):
            release.main()
        self.assertFalse(self.backend.release(self.value["tag"])["draft"])
        self.assertIsNone(self.backend.release(self.storage))
        self.assertEqual(self.backend.load(self.value["tag"])["status"], "complete")
        edits = [op for op in self.backend.operations if op[:3] == ("gh", "release", "edit")]
        self.assertEqual(len(edits), 1)
        self.assertEqual(edits[0][3], self.value["tag"])
        self.assertIn("--notes-file", edits[0])

    def test_completed_historical_release_is_not_edited_on_retry(self):
        self.presentation()
        self.value['status'] = 'complete'; self.backend.save(self.value)
        self.backend.items[self.value['tag']]['draft'] = False
        self.backend.cleanup(self.data)
        path = self.root / 'version.json'; path.write_text(json.dumps(self.data))
        for command in ['promote', 'finalize']:
            count = len(self.backend.operations)
            with patch.object(release, 'Backend', return_value=self.backend), \
                 patch.object(release, 'preflight', return_value=None), \
                 patch.object(release.version, 'git', side_effect=['tag', self.data['sourceRevision']]), \
                 patch.dict(release.os.environ, GITHUB_EVENT_NAME='push', GITHUB_REF='refs/tags/' + self.value['tag'], GITHUB_SHA=self.data['sourceRevision']), \
                 patch.object(release.sys, 'argv', ['release.py', command, '--apply', '--repository', self.backend.repository,
                    '--metadata', str(path), '--directory', str(self.root)]), patch('builtins.print'):
                release.main()
            self.assertEqual(count, len(self.backend.operations))

    def complete(self):
        self.presentation()
        self.value['status'] = 'complete'; self.backend.save(self.value)
        self.backend.items[self.value['tag']]['draft'] = False

    def test_cleanup_is_dry_run_by_default_and_only_deletes_internal_draft(self):
        self.complete()
        public = copy.deepcopy(self.backend.items[self.value['tag']])
        public_bytes = {k: v for k, v in self.backend.bytes.items() if k[0] == self.value['tag']}
        self.backend.items['unrelated'] = dict(id=99, tag_name='unrelated', draft=True, assets=[])
        count = len(self.backend.operations)
        self.backend.apply = False
        self.assertTrue(self.backend.cleanup(self.data)['dryRun'])
        self.assertEqual(len(self.backend.operations), count)
        self.backend.apply = True
        self.assertTrue(self.backend.cleanup(self.data)['deleted'])
        self.assertIsNone(self.backend.release(self.storage))
        self.assertIn('unrelated', self.backend.items)
        self.assertEqual(self.backend.items[self.value['tag']], public)
        self.assertTrue(all(self.backend.bytes[k] == v for k, v in public_bytes.items()))
        self.assertEqual(self.backend.cleanup(self.data)['reason'], 'already-clean')
        self.assertEqual(len(self.backend.operations), count + 1)

    def test_cleanup_requires_published_release_and_completed_matching_journal(self):
        self.presentation()
        with self.assertRaisesRegex(ValueError, 'require a published release'):
            self.backend.cleanup(self.data)
        self.backend.items[self.value['tag']]['draft'] = False
        with self.assertRaisesRegex(ValueError, 'keep recovery files'):
            self.backend.cleanup(self.data)
        self.value['status'] = 'complete'
        for field, replacement in [('indexes', dict(agent='sha256:' + 'f' * 64, core=self.value['indexes']['core'])),
                                   ('metadata', dict(self.data, sourceRevision='b' * 40)), ('steps', {})]:
            with self.subTest(field=field):
                original = self.value[field]
                self.value[field] = replacement; self.backend.save(self.value)
                with self.assertRaisesRegex(ValueError, 'keep recovery files'):
                    self.backend.cleanup(self.data)
                self.value[field] = original
        self.assertIsNotNone(self.backend.release(self.storage))
        self.assertFalse(any(op[-1] == 'DELETE' for op in self.backend.operations))

    def test_corrupt_public_manifest_never_allows_cleanup_or_completed_retry(self):
        self.complete()
        key = self.value['tag'], 'release-manifest.json'
        self.backend.bytes[key] += b' '
        for action in [lambda: self.backend.cleanup(self.data),
                       lambda: self.backend.prepare(self.data)]:
            with self.assertRaisesRegex(ValueError, 'checksum differs'):
                action()
        self.assertIsNotNone(self.backend.release(self.storage))

    def test_interrupted_cleanup_retries_without_rebuilding_or_republishing(self):
        self.complete()
        self.backend.fail_delete = True
        with self.assertRaisesRegex(RuntimeError, 'Interrupted cleanup'):
            self.backend.cleanup(self.data)
        self.assertIsNotNone(self.backend.release(self.storage))
        self.backend.fail_delete = False
        count = len(self.backend.operations)
        self.assertTrue(self.backend.prepare(self.data))
        self.assertEqual(len(self.backend.operations), count + 1)
        self.assertIsNone(self.backend.release(self.storage))
        count = len(self.backend.operations)
        self.assertTrue(self.backend.prepare(self.data))
        self.assertEqual(len(self.backend.operations), count)
        with self.assertRaisesRegex(ValueError, 'already published'):
            release.recover(self.backend, self.value['tag'], self.root / 'retry')

    def test_completed_manifest_remains_available_without_recovery_artifacts(self):
        self.complete(); self.backend.cleanup(self.data)
        completed = self.backend.load(self.value['tag'])
        self.assertEqual(completed['indexes'], self.value['indexes'])
        self.assertTrue(completed['publicManifest'])
    def test_missing_signature_checkpoint_blocks_cleanup_even_with_valid_checksum(self):
        self.complete()
        key = self.value['tag'], 'release-manifest.json'
        manifest = json.loads(self.backend.bytes[key]); manifest['verifiedSignatures'] = {}
        self.backend.bytes[key] = json.dumps(manifest).encode()
        self.backend.bytes[self.value['tag'], 'SHA256SUMS'] = (hashlib.sha256(self.backend.bytes[key]).hexdigest() + '  release-manifest.json\n').encode()
        with self.assertRaisesRegex(ValueError, 'Missing verified signature checkpoint'):
            self.backend.cleanup(self.data)
        self.assertIsNotNone(self.backend.release(self.storage))

    def test_manual_cleanup_command_requires_apply_for_deletion(self):
        self.complete()
        path = self.root / 'version.json'; path.write_text(json.dumps(self.data))
        for apply in [False, True]:
            self.backend.apply = apply
            args = ['release.py', 'cleanup', '--repository', self.backend.repository,
                    '--metadata', str(path), '--directory', str(self.root)]
            with patch.object(release, 'Backend', return_value=self.backend), \
                 patch.object(release.sys, 'argv', args + (['--apply'] if apply else [])), \
                 patch('builtins.print'):
                release.main()
            self.assertEqual(self.backend.release(self.storage) is None, apply)
