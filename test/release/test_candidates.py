import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import call, patch

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('candidates',ROOT/'src/tools/release/candidates.py')
candidates=importlib.util.module_from_spec(spec)
spec.loader.exec_module(candidates)


class CandidateIdentityTests(unittest.TestCase):
    def test_upgrade_descriptor_is_read_from_core_without_network_or_database(self):
        metadata = dict(displayVersion='1.2.3')
        descriptor = candidates.release.expected_upgrade_descriptor(metadata)
        with patch.object(candidates.release.version, 'run', return_value=json.dumps(descriptor)) as run:
            self.assertEqual(candidates.check_upgrade_descriptor('core-candidate', metadata), descriptor)
            run.assert_called_once_with('docker', 'run', '--rm', '--network=none', 'core-candidate', 'upgrade-descriptor-json')
        for field, value in [('schemaVersion', 2), ('product', 'other'), ('targetVersion', '1.2.4'),
                             ('targetCatalogSha256', 'a'*64), ('installerFormat', 2), ('updateProtocol', 2),
                             ('sources', [{'version':'1.2.2'}])]:
            with self.subTest(field=field), patch.object(candidates.release.version, 'run',
                    return_value=json.dumps(dict(descriptor, **{field:value}))):
                with self.assertRaisesRegex(ValueError, 'upgrade descriptor'):
                    candidates.check_upgrade_descriptor('core-candidate', metadata)

    def test_acceptance_checks_current_pair(self):
        with patch.object(candidates.subprocess, 'run') as run:
            candidates.run_acceptance('core-image', 'agent-image')
            self.assertEqual(run.call_args_list, [
                call(['bash', 'test/scripts/test-agent-acceptance.sh', 'core-image', 'agent-image'], check=True),
                call([candidates.sys.executable, 'test/scripts/test-update-registration.py', 'core-image'], check=True),
            ])

    def test_failed_update_registration_blocks_candidate_acceptance(self):
        failure = candidates.subprocess.CalledProcessError(1, 'update-registration')
        with patch.object(candidates.subprocess, 'run', side_effect=[None, failure]):
            with self.assertRaises(candidates.subprocess.CalledProcessError):
                candidates.run_acceptance('core-image', 'agent-image')

    def test_display_informational_revision_and_loaded_artifact_checked_independently(self):
        metadata=dict(productVersion='1.2.3',displayVersion='1.2.3-dev.5.g0123456789ab',
                      informationalVersion='1.2.3-dev.5.g0123456789ab+height.5.sha.'+'a'*40,
                      sourceRevision='a'*40)
        binary=dict(version=metadata['displayVersion'],informationalVersion=metadata['informationalVersion'],protocolVersion=2)
        image=dict(Architecture='amd64',Id='sha256:'+'b'*64,Config={'Labels':{
            'org.opencontainers.image.version':metadata['productVersion'],
            'org.opencontainers.image.revision':metadata['sourceRevision'],
            'com.citadel.informational-version':metadata['informationalVersion']}})
        with patch.object(candidates.release.version,'run',side_effect=[json.dumps(binary),json.dumps([image])]):
            candidates.check_image('candidate',metadata,'amd64',image['Id'])
        for failure in ['display','informational','revision','archive','architecture','protocol']:
            with self.subTest(failure=failure):
                actual=copy.deepcopy(binary); inspected=copy.deepcopy(image)
                if failure=='display': actual['version']='1.2.3'
                if failure=='informational': actual['informationalVersion']='1.2.3'
                if failure=='revision': inspected['Config']['Labels']['org.opencontainers.image.revision']+='-dirty'
                if failure=='archive': inspected['Id']='sha256:'+'c'*64
                if failure=='architecture': inspected['Architecture']='arm64'
                if failure=='protocol': actual['protocolVersion']=1
                with patch.object(candidates.release.version,'run',side_effect=[json.dumps(actual),json.dumps([inspected])]):
                    with self.assertRaises(ValueError): candidates.check_image('candidate',metadata,'amd64',image['Id'])
