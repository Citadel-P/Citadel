"""Update activation orchestration against disposable files and a simulated daemon."""
import argparse
import copy
import importlib.util
import json
import os
from pathlib import Path
import stat
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
            (self.state / 'installation.json').write_text('registration-fixture')
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

    def test_existing_setting_without_running_activation_requires_explicit_recovery(self):
        self.env.write_text(self.env.read_text() + f'{setup.SETTING}={self.state}\n')
        with self.assertRaisesRegex(setup.SetupError, 'not using it'):
            setup.enable(self.args)
        self.assertNotIn('Core restart', self.steps())
        self.assertNotIn('Registration refresh', self.steps())

    def test_docker_errors_do_not_expose_credentials(self):
        with patch.object(setup.subprocess, 'run', side_effect=setup.subprocess.CalledProcessError(
                1, ['docker'], output='secret-output', stderr='secret-password')):
            with self.assertRaises(setup.SetupError) as caught:
                self.real_run(['docker'], cwd=self.directory, environment={}, step='Preflight')
            self.assertNotIn('secret', str(caught.exception))


if __name__ == '__main__':
    unittest.main()
