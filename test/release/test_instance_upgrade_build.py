"""Runtime-compatible lifecycle compilation, packaging and failure cleanup."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location(
    'local_upgrade', ROOT / 'test/scripts/test-instance-upgrade-local.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class LifecycleBuildTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.source = self.directory / 'source checkout'
        self.target = self.source / 'target/upgrade-fixture'
        self.binary = self.target / 'debug/deps/instance_upgrade_fixture-test'
        self.binary.parent.mkdir(parents=True)
        self.binary.write_bytes(b'container-compiled executable')
        self.artifact = json.dumps({
            'reason': 'compiler-artifact', 'target': {'name': 'instance_upgrade_fixture'},
            'executable': '/source/target/debug/deps/instance_upgrade_fixture-test',
        }).encode()
        self.identity = {'version': runner.SOURCE, 'qualificationBuild': True}
        self.commands = []

    def docker(self, args, **kwargs):
        self.commands.append(args[len(runner.registration.DOCKER):])
        return subprocess.CompletedProcess(args, 0, json.dumps(self.identity).encode(), b'')

    def build(self):
        return runner.build('runtime:local', runner.SOURCE, self.directory, 'a' * 64,
                            source_root=self.source)

    def test_pinned_builder_uses_selected_checkout_and_maps_its_artifact(self):
        with patch.object(runner.registration, 'run', side_effect=self.docker), \
             patch.object(runner.subprocess, 'run', return_value=
                          subprocess.CompletedProcess([], 0, self.artifact, b'')) as compile:
            image = self.build()
        args = compile.call_args.args[0]
        self.assertEqual(args[:len(runner.registration.DOCKER)], runner.registration.DOCKER)
        self.assertIn(f'type=bind,src={self.target},dst=/source/target', args)
        self.assertIn('CARGO_HOME=/source/target/.cargo', args)
        self.assertIn('CITADEL_VERSION=' + runner.SOURCE, args)
        self.assertIn('--locked', args)
        builder = args[args.index('--name') + 1]
        self.assertIn(['build', '--builder', 'default', '--file', str(self.source / 'Dockerfile'),
                       '--target', 'rust-source', '--tag', builder, str(self.source)], self.commands)
        self.assertIn(['rm', '-fv', builder], self.commands)
        self.assertIn(['image', 'rm', builder], self.commands)
        self.assertEqual((self.directory / runner.SOURCE / 'citadel-server').read_bytes(), self.binary.read_bytes())
        self.assertIn(['run', '--rm', '--network=none', image, 'version-json'], self.commands)
        self.assertTrue(self.binary.exists(), 'Compiler cache must survive cleanup')

    def test_compile_timeout_removes_builder_without_packaging_or_starting_a_fixture(self):
        with patch.object(runner.registration, 'run', side_effect=self.docker), \
             patch.object(runner.subprocess, 'run', side_effect=subprocess.TimeoutExpired('docker', 1800)):
            with self.assertRaises(subprocess.TimeoutExpired):
                self.build()
        builder = self.commands[0][self.commands[0].index('--tag') + 1]
        self.assertEqual(self.commands[-2:], [['rm', '-fv', builder], ['image', 'rm', builder]])
        self.assertFalse((self.directory / runner.SOURCE).exists())

    def test_wrong_packaged_identity_is_removed(self):
        for identity in [{'version': runner.SOURCE}, {'version': runner.TARGET, 'qualificationBuild': True}]:
            with self.subTest(identity=identity), tempfile.TemporaryDirectory() as temporary:
                self.directory = Path(temporary)
                self.identity = identity
                with patch.object(runner.registration, 'run', side_effect=self.docker), \
                     patch.object(runner.subprocess, 'run', return_value=
                                  subprocess.CompletedProcess([], 0, self.artifact, b'')):
                    with self.assertRaisesRegex(AssertionError, 'incorrect build identity'):
                        self.build()
                self.assertEqual(self.commands[-1][:2], ['image', 'rm'])
                self.assertTrue(self.commands[-1][2].startswith('citadel-upgrade-fixture:'))

    def test_packaged_startup_failure_is_fatal_and_removes_the_image(self):
        def docker(args, **kwargs):
            result = self.docker(args, **kwargs)
            if args[-1] == 'version-json':
                raise RuntimeError('Fixture command failed')
            return result
        with patch.object(runner.registration, 'run', side_effect=docker), \
             patch.object(runner.subprocess, 'run', return_value=
                          subprocess.CompletedProcess([], 0, self.artifact, b'')):
            with self.assertRaisesRegex(RuntimeError, 'Fixture command failed'):
                self.build()
        self.assertEqual(self.commands[-1][:2], ['image', 'rm'])
        self.assertTrue(self.commands[-1][2].startswith('citadel-upgrade-fixture:'))


if __name__ == '__main__':
    unittest.main()
