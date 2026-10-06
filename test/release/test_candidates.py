import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('candidates',ROOT/'src/tools/release/candidates.py')
candidates=importlib.util.module_from_spec(spec)
spec.loader.exec_module(candidates)


class CandidateIdentityTests(unittest.TestCase):
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
