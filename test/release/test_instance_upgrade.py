import copy
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('upgrade_fixture', ROOT / 'test/scripts/test-instance-upgrade.py')
upgrade = importlib.util.module_from_spec(spec)
spec.loader.exec_module(upgrade)


class PublishedUpgradeTests(unittest.TestCase):
    def setUp(self):
        self.source = dict(targetVersion='0.1.1', targetCatalogSha256='a' * 64)
        self.target = dict(descriptor=dict(sources=[dict(
            version='0.1.1', catalogSha256='a' * 64, architectures=['amd64'])]))

    def test_empty_ambiguous_and_mismatched_sources_are_rejected(self):
        upgrade.require_eligible(self.source, self.target, 'amd64')
        for change in ['empty', 'duplicate', 'version', 'catalog', 'architecture']:
            with self.subTest(change=change):
                target = copy.deepcopy(self.target)
                source = target['descriptor']['sources'][0]
                if change == 'empty':
                    target['descriptor']['sources'] = []
                elif change == 'duplicate':
                    target['descriptor']['sources'].append(copy.deepcopy(source))
                elif change == 'version':
                    source['version'] = '0.1.0'
                elif change == 'catalog':
                    source['catalogSha256'] = 'b' * 64
                else:
                    source['architectures'] = ['arm64']
                with self.assertRaisesRegex(AssertionError, 'does not allow'):
                    upgrade.require_eligible(self.source, target, 'amd64')

    def test_ineligible_pair_stops_before_installation_or_approval(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = upgrade.UpgradeFixture('ghcr.io/citadel-p/citadel@sha256:' + 'b' * 64, Path(directory))
            with patch.object(fixture, 'json_command', side_effect=[
                [{'Architecture': 'amd64'}], self.source, {'image': fixture.core},
                {'descriptor': {'sources': []}},
            ]), patch.object(fixture, 'execute') as execute, patch.object(fixture, 'http') as http:
                with self.assertRaisesRegex(AssertionError, 'does not allow'):
                    fixture.run_transition('0.1.2')
                execute.assert_not_called()
                http.assert_not_called()

    def test_mutable_or_unofficial_source_creates_no_daemon(self):
        for image in ['citadel:latest', 'ghcr.io/citadel-p/citadel:0.1.1',
                      'example.test/citadel@sha256:' + 'a' * 64]:
            with self.subTest(image=image), tempfile.TemporaryDirectory() as directory:
                fixture = upgrade.UpgradeFixture(image, Path(directory))
                with patch.object(fixture, 'host') as host:
                    with self.assertRaisesRegex(AssertionError, 'exact official Core digest'):
                        fixture.start(published=True)
                    host.assert_not_called()

    def test_published_fixture_pulls_digest_and_keeps_network_but_default_stays_offline(self):
        for published in [False, True]:
            with self.subTest(published=published), tempfile.TemporaryDirectory() as directory:
                image = 'ghcr.io/citadel-p/citadel@sha256:' + 'b' * 64 if published else 'local-candidate'
                fixture = upgrade.UpgradeFixture(image, Path(directory))
                calls = []
                def host(*args, **kwargs):
                    calls.append(args)
                    if args[:2] == ('image', 'save'):
                        Path(args[3]).write_bytes(b'fixture archive')
                    return subprocess.CompletedProcess([], 0, b'fixture-id\n', b'')
                with patch.object(fixture, 'host', side_effect=host):
                    fixture.start(published=published)
                disconnected = any(c[:2] == ('network', 'disconnect') for c in calls)
                saved = any(c[:2] == ('image', 'save') for c in calls)
                pulled = any(c[-2:] == ('pull', image) for c in calls)
                self.assertEqual((disconnected, saved, pulled), (not published, not published, published))
                self.assertEqual(fixture.network is not None, published)
                if published:
                    self.assertEqual(fixture.tag, image)

    def test_refresh_can_change_only_core_and_configuration(self):
        before = dict(coreContainerId='source', configurationSha256='a', runtimeSha256='b',
                      instanceId='instance', id='registration', databaseContainerId='pg',
                      coreVolume='core', databaseVolume='db', network='net', daemonId='daemon')
        after = dict(before, coreContainerId='target', configurationSha256='c', runtimeSha256='d')
        upgrade.require_preserved(before, after)
        for field in ['instanceId', 'id', 'databaseContainerId', 'coreVolume', 'databaseVolume', 'network', 'daemonId']:
            with self.subTest(field=field), self.assertRaisesRegex(AssertionError, 'identity changed'):
                upgrade.require_preserved(before, dict(after, **{field: 'other'}))
        with self.assertRaisesRegex(AssertionError, 'not replaced'):
            upgrade.require_preserved(before, before)

    def test_database_identity_start_time_and_restart_count_are_checked(self):
        before = dict(Id='pg', State=dict(StartedAt='original'), RestartCount=0)
        for after in [dict(before, Id='other'), dict(before, State=dict(StartedAt='later')),
                      dict(before, RestartCount=1)]:
            self.assertNotEqual(upgrade.database_state(before), upgrade.database_state(after))

    def test_ambiguous_http_mutation_is_not_retried_and_output_is_redacted(self):
        fixture = upgrade.UpgradeFixture('unused', Path('/unused'))
        result = subprocess.CompletedProcess([], 1, b'private-response', b'private-error')
        with patch.object(fixture, 'execute', return_value=result) as execute:
            with self.assertRaisesRegex(AssertionError, 'response withheld') as failure:
                fixture.http('updates/approve', {'requestId': 'one-request'}, 'private-token')
            execute.assert_called_once()
            self.assertNotIn('private', str(failure.exception))

    def test_exited_helper_removed_during_finalization_is_not_a_runner_failure(self):
        fixture = upgrade.UpgradeFixture('unused', Path('/unused'))
        listed = subprocess.CompletedProcess([], 0, b'helper-id\n', b'')
        for result, allowed in [
            (subprocess.CompletedProcess([], 1, b'', b'Error: No such object: helper-id'), True),
            (subprocess.CompletedProcess([], 1, b'', b'daemon unavailable'), False),
            (subprocess.CompletedProcess([], 0, b'[{"State":{"ExitCode":0}}]', b''), True),
            (subprocess.CompletedProcess([], 0, b'[{"State":{"ExitCode":1}}]', b''), False),
        ]:
            with self.subTest(result=result), patch.object(fixture, 'docker', side_effect=[listed, result]):
                if allowed:
                    fixture.check_exited_helpers('operation')
                else:
                    with self.assertRaises(AssertionError):
                        fixture.check_exited_helpers('operation')


class LocalUpgradeTests(unittest.TestCase):
    def test_browser_publishes_only_loopback_ui_and_retains_its_own_network(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = upgrade.registration.Fixture('local-core', Path(directory))
            calls = []
            def host(*args, **kwargs):
                calls.append(args)
                if args[:2] == ('image', 'save'):
                    Path(args[3]).write_bytes(b'archive')
                return subprocess.CompletedProcess([], 0, b'fixture-id\n', b'')
            with patch.object(fixture, 'host', side_effect=host):
                fixture.start(registry=True, browser_port=18080)
                create = next(c for c in calls if c[0] == 'create')
                self.assertEqual(create[create.index('--publish') + 1], '127.0.0.1:18080:18000')
                self.assertEqual(create.count('--publish'), 1)
                self.assertFalse(any(c[:2] == ('network', 'disconnect') for c in calls))
                self.assertIsNotNone(fixture.network)
                fixture.close()
                self.assertIn(('rm', '-fv', 'fixture-id'), calls)
                self.assertIn(('network', 'rm', 'fixture-id'), calls)

    def test_browser_setup_leaves_check_review_and_approval_to_the_user(self):
        spec = importlib.util.spec_from_file_location(
            'browser_upgrade', ROOT / 'test/scripts/test-instance-upgrade-local.py')
        local = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(local)
        fixture = local.LocalUpgrade('unused', Path('/unused'))
        with patch.object(fixture, 'inspect_transition') as inspect, \
             patch.object(fixture, 'setup_installation', return_value=({}, 'pg-secret', 'admin-secret')), \
             patch.object(fixture, 'http') as http, \
             patch.object(local.time, 'sleep', side_effect=KeyboardInterrupt), \
             patch('sys.stdout', new_callable=io.StringIO) as output:
            with self.assertRaises(KeyboardInterrupt):
                fixture.serve_browser(18080)
            inspect.assert_called_once_with(local.TARGET)
            http.assert_not_called()
            self.assertIn('http://localhost:18080/updates', output.getvalue())
            self.assertIn('admin-secret', output.getvalue())
            self.assertNotIn('pg-secret', output.getvalue())

    def test_private_registry_and_published_mode_cannot_be_combined(self):
        fixture = upgrade.registration.Fixture('unused', Path('/unused'))
        with patch.object(fixture, 'host') as host:
            with self.assertRaisesRegex(AssertionError, 'cannot be used'):
                fixture.start(published=True, registry=True)
            host.assert_not_called()

    def test_private_push_selects_the_official_digest_among_local_aliases(self):
        spec = importlib.util.spec_from_file_location(
            'local_upgrade', ROOT / 'test/scripts/test-instance-upgrade-local.py')
        local = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(local)
        fixture = local.LocalUpgrade('unused', Path('/unused'))
        digest = 'ghcr.io/citadel-p/citadel@sha256:' + 'a' * 64
        with patch.object(fixture, 'docker'), patch.object(fixture, 'json_command', return_value=[
                {'RepoDigests': ['citadel-upgrade-fixture@sha256:' + 'b' * 64, digest]}]):
            self.assertEqual(fixture.publish_private('image', 'target'), digest)
        with patch.object(fixture, 'docker'), patch.object(fixture, 'json_command', return_value=[
                {'RepoDigests': ['unrelated@sha256:' + 'b' * 64]}]):
            with self.assertRaisesRegex(AssertionError, 'Unexpected private digest'):
                fixture.publish_private('image', 'target')


if __name__ == '__main__':
    unittest.main()
