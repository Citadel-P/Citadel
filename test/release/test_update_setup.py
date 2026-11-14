"""Update activation orchestration against disposable files and a simulated daemon."""
import argparse
import copy
import importlib.util
import json
import os
from pathlib import Path
import stat
import fcntl
import tempfile
import types
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('update_setup', ROOT / 'deploy/install/enable-updates.py')
setup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup)


class UpdateSetupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.directory = self.root / 'citadel'
        self.directory.mkdir(mode=0o700)
        self.env = self.directory / '.env'
        self.env.write_text('# operator settings\nPG_PASSWORD=secret-fixture\nCITADEL_IMAGE=installed\n')
        self.env.chmod(0o600)
        self.compose = self.directory / 'docker-compose.yml'
        self.compose.write_text('services: {}\n')
        self.state = self.root / 'update-state'
        self.socket = f'/run/user/{os.geteuid()}/docker.sock'
        self.args = argparse.Namespace(directory=str(self.directory), project='existing-project',
                                       compose_file=[], state_directory=str(self.state))
        self.endpoint = 'unix://' + self.socket
        self.operating_system = 'Linux'
        self.calls = []
        self.fail = None
        self.restarted = False
        self.after_preflight = None
        self.image = 'sha256:' + 'a' * 64
        self.core = {'Id': 'core-before', 'Image': self.image, 'Config': {'Env': [], 'Image': 'installed'}}
        self.database = {'Id': 'database-unchanged', 'Image': 'postgres-installed', 'State': {'StartedAt': 'original'}, 'RestartCount': 0}
        self.runtime_image = self.image
        self.selected_image = self.image
        self.existing = ''
        self.identity_checks = 0
        self.ready_after = 1
        original_stat = Path.stat
        def stat_path(path, *args, **kwargs):
            if str(path) == self.socket:
                return types.SimpleNamespace(st_mode=stat.S_IFSOCK)
            return original_stat(path, *args, **kwargs)
        self.addCleanup(patch.stopall)
        patch.object(Path, 'stat', stat_path).start()
        self.real_run = setup.run
        patch.object(setup, 'run', self.command).start()
        patch.dict(os.environ, {'DOCKER_CONTEXT': 'fixture', 'CITADEL_IMAGE': 'must-not-leak',
                                'PG_PASSWORD': 'must-not-leak', 'COMPOSE_FILE': 'wrong.yml'}).start()

    def command(self, args, *, cwd, environment, step, timeout=180):
        self.calls.append((args, step, environment.copy()))
        if step == self.fail:
            raise setup.SetupError('fixture failure')
        if step == 'Existing installation inspection':
            return self.existing
        if step == 'Compose project inspection':
            return json.dumps({'name':'existing-project'})
        if step == 'First-run setup check':
            self.identity_checks += 1
            if self.identity_checks < self.ready_after:
                raise setup.SetupError('not initialized yet')
            return json.dumps({'instanceId': '01900000-0000-7000-8000-000000000001'})
        if step == 'Docker context inspection':
            return self.endpoint
        if step == 'Docker inspection':
            return json.dumps({'OSType': 'linux', 'OperatingSystem': self.operating_system,
                               'SecurityOptions': ['name=rootless'], 'DockerRootDir': str(self.root / 'docker')})
        if step == 'Compose service inspection':
            return args[-1]
        if step == 'Container inspection':
            if args[-1] == 'pg_db':
                return json.dumps([self.database])
            core = copy.deepcopy(self.core)
            if self.restarted:
                core.update(Id='core-after', Image=self.runtime_image)
                core['Config']['Env'] = [f'{setup.SETTING}={self.state}']
            return json.dumps([core])
        if step == 'Installed image verification':
            return self.selected_image
        if step == 'Installation registration':
            if not (self.state / 'installation.json').exists():
                (self.state / 'installation.json').write_text('registration-fixture')
                (self.state / 'installation.lock').touch(mode=0o600)
        if step == 'Installation and recovery preflight' and self.after_preflight:
            self.after_preflight()
        if step == 'Core restart':
            self.restarted = True
        if step == 'Setup helper inspection':
            failed = next(args for args, name, _ in reversed(self.calls) if name == self.fail)
            return failed[failed.index('--label') + 1].split('=', 1)[1]
        return '{}'

    def steps(self):
        return [step for _, step, _ in self.calls]

    def test_automatic_compose_setup_does_not_create_a_second_registration(self):
        original = self.command
        def automatic(arguments, **kwargs):
            if kwargs.get('step') == 'Compose project inspection':
                return json.dumps({'name': 'existing-project', 'services': {'server': {'environment': {
                    'CITADEL_UPDATE_STATE_DIRECTORY': '/var/lib/citadel-updates'}}}})
            return original(arguments, **kwargs)
        with patch.object(setup, 'run', side_effect=automatic):
            with self.assertRaisesRegex(setup.SetupError, 'configures updates automatically'):
                setup.enable(self.args)

    def test_enables_after_preflight_restarts_only_core_and_verifies(self):
        original = self.env.read_bytes()
        self.assertIn('configured', setup.enable(self.args))
        self.assertEqual(self.env.read_bytes(), original + f'{setup.SETTING}={self.state}\n'.encode())
        self.assertEqual(stat.S_IMODE(self.env.stat().st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(self.state.stat().st_mode), 0o700)
        self.assertLess(self.steps().index('Installation and recovery preflight'), self.steps().index('Core restart'))
        self.assertEqual(self.steps()[-2:], ['Registration refresh', 'Final installation verification'])
        restart = next(args for args, step, _ in self.calls if step == 'Core restart')
        self.assertEqual(restart[-10:], ['up', '-d', '--no-deps', '--no-build', '--pull', 'never', '--wait', '--wait-timeout', '180', 'server'])
        helper = next(args for args, step, _ in self.calls if step == 'Installation registration')
        self.assertEqual(helper[:4], ['docker', '--host', 'unix://' + self.socket, 'run'])
        self.assertEqual(helper[helper.index('--user') + 1], '0:0')
        self.assertEqual(helper[helper.index('--network') + 1], 'none')
        self.assertIn(self.image, helper)
        self.assertIn(f'type=bind,src={self.directory},dst={self.directory},readonly', helper)
        for _, _, environment in self.calls:
            self.assertNotIn('PG_PASSWORD', environment)
            self.assertNotIn('CITADEL_IMAGE', environment)
            self.assertNotIn('COMPOSE_FILE', environment)

    def test_preserves_overlay_order_and_existing_project(self):
        overlay = self.directory / 'compose.edge.yml'
        overlay.write_text('services: {}\n')
        self.args.compose_file = [str(self.compose), str(overlay)]
        setup.enable(self.args)
        register = next(args for args, step, _ in self.calls if step == 'Installation registration')
        self.assertEqual(register[-4:], ['--compose-file', str(self.compose), '--compose-file', str(overlay)])
        self.assertIn('existing-project', register)

    def test_registration_failure_never_changes_env_or_restarts_core(self):
        original = self.env.read_bytes()
        self.fail = 'Installation registration'
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args)
        self.assertEqual(self.env.read_bytes(), original)
        self.assertNotIn('Core restart', self.steps())
        self.assertIn('Setup helper cleanup', self.steps())

    def test_preflight_failure_preserves_registration_without_enabling(self):
        self.fail = 'Installation and recovery preflight'
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args)
        self.assertNotIn(setup.SETTING, self.env.read_text())
        self.assertTrue((self.state / 'installation.json').exists())
        self.assertNotIn('Core restart', self.steps())

    def test_concurrent_env_edit_is_not_overwritten(self):
        self.after_preflight = lambda: self.env.write_text('PG_PASSWORD=edited\n')
        with self.assertRaisesRegex(setup.SetupError, 'changed during setup'):
            setup.enable(self.args)
        self.assertEqual(self.env.read_text(), 'PG_PASSWORD=edited\n')
        self.assertNotIn('Core restart', self.steps())

    def test_failed_restart_keeps_evidence_and_does_not_refresh_or_roll_back(self):
        self.fail = 'Core restart'
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args)
        self.assertIn(setup.SETTING, self.env.read_text())
        self.assertTrue((self.state / 'installation.json').exists())
        self.assertNotIn('Registration refresh', self.steps())
        self.assertEqual(self.steps().count('Core restart'), 1)

    def test_failed_refresh_does_not_retry_or_remove_registration(self):
        self.fail = 'Registration refresh'
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args)
        self.assertTrue((self.state / 'installation.json').exists())
        self.assertNotIn('Final installation verification', self.steps())
        self.assertEqual(self.steps().count('Registration refresh'), 1)

    def test_changed_core_image_refuses_registration_refresh(self):
        self.runtime_image = 'sha256:' + 'b' * 64
        with self.assertRaisesRegex(setup.SetupError, 'identity changed'):
            setup.enable(self.args)
        self.assertNotIn('Registration refresh', self.steps())

    def test_moved_local_image_tag_does_not_upgrade_core_during_setup(self):
        self.selected_image = 'sha256:' + 'c' * 64
        with self.assertRaisesRegex(setup.SetupError, 'no longer points'):
            setup.enable(self.args)
        self.assertNotIn(setup.SETTING, self.env.read_text())
        self.assertNotIn('Core restart', self.steps())

    def test_verified_repeat_does_not_restart_or_refresh(self):
        setup.enable(self.args)
        self.calls.clear()
        self.assertIn('already configured', setup.enable(self.args))
        self.assertNotIn('Core restart', self.steps())
        self.assertNotIn('Registration refresh', self.steps())

    def test_foreign_state_setting_is_not_replaced(self):
        self.env.write_text(f'{setup.SETTING}=/another-state\n')
        with self.assertRaisesRegex(setup.SetupError, 'differs'):
            setup.enable(self.args)
        self.assertFalse(self.calls)

    def test_unsafe_or_symlinked_env_is_rejected(self):
        self.env.chmod(0o644)
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args)
        self.env.unlink()
        target = self.directory / 'private'
        target.write_text('PG_PASSWORD=secret\n')
        target.chmod(0o600)
        self.env.symlink_to(target)
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args)
        self.assertFalse(self.calls)

    def test_remote_daemon_is_refused_before_registration(self):
        self.endpoint = 'ssh://remote.example'
        with self.assertRaisesRegex(setup.SetupError, 'local Linux Docker socket'):
            setup.enable(self.args)
        self.assertNotIn('Installation registration', self.steps())
        self.assertFalse(self.state.exists())

    def test_docker_desktop_is_not_treated_as_a_native_installation(self):
        self.operating_system = 'Docker Desktop'
        with self.assertRaisesRegex(setup.SetupError, 'native Linux'):
            setup.enable(self.args)
        self.assertFalse(self.state.exists())

    def test_private_state_permissions_are_not_silently_changed(self):
        self.state.mkdir(mode=0o755)
        # Exercise public permissions even when earlier deployment tests set umask 077.
        self.state.chmod(0o755)
        with self.assertRaisesRegex(setup.SetupError, '0700'):
            setup.enable(self.args)
        self.assertEqual(stat.S_IMODE(self.state.stat().st_mode), 0o755)
        self.assertNotIn('Installation registration', self.steps())

    def test_state_symlink_cannot_redirect_write_probe(self):
        target = self.root / 'another-state'
        target.mkdir(mode=0o700)
        self.state.symlink_to(target, target_is_directory=True)
        with patch.object(setup.tempfile, 'TemporaryFile') as probe, \
             self.assertRaisesRegex(setup.SetupError, 'no symlinks'):
            setup.prepare_state_directory(self.state)
        probe.assert_not_called()
        self.assertEqual(list(target.iterdir()), [])

    def test_state_write_probe_preserves_existing_evidence(self):
        self.state.mkdir(mode=0o700)
        evidence = self.state / 'installation.json'
        evidence.write_text('private registration')
        setup.prepare_state_directory(self.state)
        self.assertEqual(evidence.read_text(), 'private registration')
        self.assertEqual(list(self.state.iterdir()), [evidence])

    def test_existing_setting_without_running_activation_requires_explicit_recovery(self):
        self.env.write_text(self.env.read_text() + f'{setup.SETTING}={self.state}\n')
        with self.assertRaisesRegex(setup.SetupError, 'not using it'):
            setup.enable(self.args)
        self.assertNotIn('Core restart', self.steps())
        self.assertNotIn('Registration refresh', self.steps())

    def test_start_waits_for_account_then_activates_both_updaters(self):
        self.args.start = True
        self.args.project = None
        self.ready_after = 3
        with patch.object(setup.time, 'sleep') as sleep:
            setup.enable(self.args)
        self.assertEqual(sleep.call_count, 2)
        steps = self.steps()
        self.assertLess(steps.index('Installation startup'), steps.index('First-run setup check'))
        self.assertLess(steps.index('First-run setup check'), steps.index('Installation registration'))
        self.assertIn('Final installation verification', steps)

    def test_start_existing_installation_does_not_pull_or_replace_before_registration(self):
        self.args.start = True
        self.existing = 'existing-core\nexisting-db'
        setup.enable(self.args)
        self.assertNotIn('Installation image download', self.steps())
        self.assertNotIn('Installation startup', self.steps())
        self.assertEqual(self.steps().count('Core restart'), 1)

    def test_start_timeout_leaves_containers_and_env_untouched(self):
        self.args.start = True
        self.args.setup_timeout = 1
        self.ready_after = 100
        original = self.env.read_bytes()
        with patch.object(setup.time, 'monotonic', side_effect=[0, 2]):
            with self.assertRaisesRegex(setup.SetupError, 'wait expired'):
                setup.enable(self.args)
        self.assertEqual(self.env.read_bytes(), original)
        self.assertNotIn('Installation registration', self.steps())

    def test_default_path_and_project_need_no_operator_settings(self):
        self.args.state_directory = None
        self.state = self.directory.with_name(self.directory.name + '-update-state')
        self.args.project = None
        setup.enable(self.args)
        self.assertIn(f'{setup.SETTING}={self.state}', self.env.read_text())

    def test_registered_deployment_checks_ownership_stops_core_then_refreshes(self):
        setup.enable(self.args)
        self.calls.clear()
        def replace():
            self.assertEqual(self.steps()[-1], 'Deployment Core stop')
            with (self.state / 'installation.lock').open('rb') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.calls.append(([], 'replacement', {}))
        setup.enable(self.args, replace=replace)
        self.assertLess(self.steps().index('Deployment ownership check'), self.steps().index('replacement'))
        self.assertEqual(self.steps()[-2:], ['Registration refresh', 'Final installation verification'])

    def test_registered_deployment_refuses_changed_evidence(self):
        setup.enable(self.args)
        self.calls.clear()
        self.after_preflight = lambda: (self.state / 'execution.json').write_text('new evidence')
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args, replace=lambda: self.assertTrue(False, 'must not replace'))
        self.assertNotIn('Deployment Core stop', self.steps())

    def test_registered_deployment_refuses_concurrent_input_edit(self):
        setup.enable(self.args)
        self.calls.clear()
        self.after_preflight = lambda: self.compose.write_text('changed configuration')
        with self.assertRaisesRegex(setup.SetupError, 'inputs or evidence changed'):
            setup.enable(self.args, replace=lambda: self.assertTrue(False, 'must not replace'))
        self.assertNotIn('Deployment Core stop', self.steps())

    def test_registered_deployment_refuses_an_owned_update(self):
        setup.enable(self.args)
        self.calls.clear()
        self.fail = 'Deployment ownership check'
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args, replace=lambda: self.assertTrue(False, 'must not replace'))
        self.assertNotIn('Deployment Core stop', self.steps())

    def test_registered_deployment_refuses_lock_contention(self):
        setup.enable(self.args)
        self.calls.clear()
        with (self.state / 'installation.lock').open('rb') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with self.assertRaises(BlockingIOError):
                setup.enable(self.args, replace=lambda: self.assertTrue(False, 'must not replace'))
        self.assertNotIn('Deployment Core stop', self.steps())

    def test_registered_deployment_failure_is_not_refreshed_or_rolled_back(self):
        setup.enable(self.args)
        self.calls.clear()
        def replace():
            raise setup.SetupError('replacement failed')
        with self.assertRaises(setup.SetupError):
            setup.enable(self.args, replace=replace)
        self.assertNotIn('Registration refresh', self.steps())
        self.assertTrue((self.state / 'installation.json').exists())

    def test_docker_errors_do_not_expose_credentials(self):
        with patch.object(setup.subprocess, 'run', side_effect=setup.subprocess.CalledProcessError(
                1, ['docker'], output='secret-output', stderr='secret-password')):
            with self.assertRaises(setup.SetupError) as caught:
                self.real_run(['docker'], cwd=self.directory, environment={}, step='Preflight')
            self.assertNotIn('secret', str(caught.exception))


if __name__ == '__main__':
    unittest.main()
