"""Opt-in digest preservation test against two disposable local registries."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
import urllib.request
import uuid

from test_release import fixture_oci, release

REGISTRY = 'registry@sha256:325b4b29b041e82803abeb703e201655e4e23ab83264ec1a7c9ddb0a5b14a6e0'


@unittest.skipUnless(os.environ.get('CITADEL_TEST_OCI_TRANSPORT') == '1', 'Set CITADEL_TEST_OCI_TRANSPORT=1 with Docker and skopeo')
class OciTransportTests(unittest.TestCase):
    def test_two_registry_indexes_and_child_manifests_keep_their_digests(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            for arch in ['amd64','arm64']: fixture_oci(root/f'agent-{arch}.oci.tar',arch,nested=True)
            expected=release.assemble('agent',root)
            endpoints=[]
            for _ in range(2):
                name='citadel-release-transport-'+uuid.uuid4().hex[:12]
                self.addCleanup(subprocess.run,['docker','rm','-fv',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
                release.version.run('docker','run','-d','--name',name,'-p','127.0.0.1::5000',REGISTRY)
                endpoint=release.version.run('docker','port',name,'5000/tcp')
                for attempt in range(100):
                    try:
                        urllib.request.urlopen('http://'+endpoint+'/v2/',timeout=1).close()
                        break
                    except OSError:
                        if attempt==99: raise
                        time.sleep(.1)
                endpoints.append(endpoint+'/fixture')
            def skopeo(*args):
                return subprocess.check_output(['skopeo','--insecure-policy',*args])
            with release.oci_source('oci-archive:'+str(root/'agent.oci.tar')) as source:
                skopeo('copy','--all','--preserve-digests','--dest-tls-verify=false',
                       source,'docker://'+endpoints[0]+':1.2.3')
            skopeo('copy','--all','--preserve-digests','--src-tls-verify=false','--dest-tls-verify=false',
                   'docker://'+endpoints[0]+'@'+expected,'docker://'+endpoints[1]+':1.2.3')
            for repo in endpoints:
                raw=skopeo('inspect','--raw','--tls-verify=false','docker://'+repo+':1.2.3')
                self.assertEqual(release.digest(raw),expected)
                entries=json.loads(raw)['manifests']
                self.assertEqual({e['platform']['architecture'] for e in entries},{'amd64','arm64'})
                for entry in entries:
                    child=skopeo('inspect','--raw','--tls-verify=false','docker://'+repo+'@'+entry['digest'])
                    self.assertEqual(release.digest(child),entry['digest'])
                skopeo('copy','--all','--preserve-digests','--src-tls-verify=false','--dest-tls-verify=false',
                       'docker://'+repo+'@'+expected,'docker://'+repo+':1.2')
                self.assertEqual(release.digest(skopeo('inspect','--raw','--tls-verify=false','docker://'+repo+':1.2')),expected)
