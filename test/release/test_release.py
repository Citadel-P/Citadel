"""Failure/recovery tests: no registry, GitHub, tag or docs mutations."""
import copy
import importlib.util
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('release', ROOT / 'src/tools/release/release.py')
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


def metadata(product='1.2.3'):
    return dict(schemaVersion=1, productVersion=product, displayVersion=product,
                informationalVersion=f'{product}+height.5.sha.' + 'a' * 40,
                sourceRevision='a' * 40, versionHeight=5, dirty=False,
                nbgvPublicRelease=True, releaseEligible=True, identityStatus='resolved',
                toolIdentity=release.version.tool_identity())


def record(product='1.2.3'):
    return dict(schemaVersion=1, sequence=0, tag='v'+product, metadata=metadata(product),
                indexes={'agent': 'sha256:'+'a'*64, 'core': 'sha256:'+'b'*64},
                steps={'artifacts-retained': True}, artifacts={}, status='prepared',
                signingIdentity='https://example.invalid/ci', sourceRunId='42',
                compatibility=dict(bootstrap=True, agent=None, core=None, protocolVersion=2,
                                   architectures=['amd64','arm64'], agentFirstUpgrades=True))


class FakeBackend:
    repository = 'Citadel-P/Citadel'

    def __init__(self, apply=True):
        self.apply = apply
        self.images, self.records, self.assets, self.originals, self.operations = {}, {}, {}, {}, []
        self.fail = None
        self.current = None
        self.corrupt_copy = False

    def mutation(self, *operation):
        if not self.apply:
            raise AssertionError('Unexpected external mutation')
        if self.fail == operation:
            raise RuntimeError('injected failure')
        self.operations.append(operation)

    def save(self, value):
        self.mutation('save', value['status'])
        value['sequence'] += 1
        self.records[value['tag']] = copy.deepcopy(value)

    def load(self, tag):
        return copy.deepcopy(self.records.get(tag))

    def releases(self):
        return [dict(tag_name=k, draft=v['status']!='complete', assets=[{'name': n} for t,n in self.assets if t==k])
                for k,v in self.records.items()]

    def release(self, tag):
        return next((r for r in self.releases() if r['tag_name']==tag), None)

    def tags(self, repository):
        return [tag for repo,tag in self.images if repo==repository]

    def existing(self, repository, tag):
        return self.images.get((repository,tag))

    def copy(self, source, destination):
        self.mutation('copy', destination)
        repo,tag = destination.rsplit(':',1)
        if source.startswith('oci-archive:'):
            component = Path(source.removeprefix('oci-archive:')).name.split('.')[0]
            value = self.current['indexes'][component]
        else:
            value = source.split('@')[1]
        self.images[repo,tag] = 'sha256:'+'0'*64 if self.corrupt_copy else value

    def sign(self, reference, identity):
        self.mutation('sign', reference)

    def manifest(self, reference):
        return json.dumps({'manifests':[{'platform':{'os':'linux','architecture':a}} for a in ['amd64','arm64']]}).encode()

    def asset(self, rel, name, path):
        Path(path).write_bytes(self.assets[rel['tag_name'], name])

    def action_artifact(self, value, name, path):
        Path(path).write_bytes(self.originals[name])

    def upload(self, tag, path):
        self.mutation('upload', Path(path).name)
        self.assets[tag,Path(path).name] = Path(path).read_bytes()


def fixture_oci(destination, arch, nested=False):
    """A real, tiny OCI image layout without Docker/network requirements."""
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp); (root/'blobs/sha256').mkdir(parents=True)
        def blob(obj, media):
            raw=release.canonical(obj); digest=release.digest(raw)
            (root/'blobs/sha256'/digest.split(':')[1]).write_bytes(raw)
            return dict(mediaType=media, size=len(raw), digest=digest)
        config=blob(dict(architecture=arch, os='linux', config={}, rootfs={'type':'layers','diff_ids':[]}),
                    'application/vnd.oci.image.config.v1+json')
        entry=blob(dict(schemaVersion=2,mediaType='application/vnd.oci.image.manifest.v1+json',config=config,layers=[]),
                   'application/vnd.oci.image.manifest.v1+json')
        if nested:
            entry=blob(dict(schemaVersion=2, mediaType='application/vnd.oci.image.index.v1+json', manifests=[entry]),
                       'application/vnd.oci.image.index.v1+json')
        (root/'index.json').write_bytes(release.canonical(dict(schemaVersion=2,manifests=[entry])))
        (root/'oci-layout').write_text('{"imageLayoutVersion":"1.0.0"}')
        with tarfile.open(destination,'w') as out:
            for path in sorted(root.rglob('*')):
                if path.is_file(): out.add(path, arcname=str(path.relative_to(root)))


class PromotionTests(unittest.TestCase):
    def setUp(self):
        self.backend=FakeBackend(); self.record=record(); self.backend.current=self.record
        self.directory=Path('/nonexistent/candidates')

    def promote(self):
        return release.promote(self.backend,self.record,self.directory)

    def test_order_signatures_aliases_and_idempotence(self):
        self.promote()
        operations=self.backend.operations
        first_core=next(i for i,op in enumerate(operations) if op==('copy', 'ghcr.io/citadel-p/citadel:1.2.3'))
        for repo in release.POLICY['registries']['agent']:
            self.assertLess(operations.index(('sign',repo+'@'+self.record['indexes']['agent'])),first_core)
        first_alias=next(i for i,op in enumerate(operations) if op[0]=='copy' and not op[1].endswith(':1.2.3'))
        for comp in ['core','agent']:
            for repo in release.POLICY['registries'][comp]:
                self.assertLess(operations.index(('sign',repo+'@'+self.record['indexes'][comp])),first_alias)
                self.assertEqual(self.backend.existing(repo,'1.2.3'), self.record['indexes'][comp])
        self.assertNotIn(('ghcr.io/citadel-p/citadel.agent','latest'),self.backend.images)
        self.backend.operations=[]
        self.promote()
        self.assertFalse(any(op[0]=='copy' and op[1].endswith(':1.2.3') for op in self.backend.operations))

    def test_partial_registry_signature_core_and_alias_failures_resume(self):
        for failure in [('copy','docker.io/citadelplane/citadel-agent:1.2.3'),
                        ('sign','docker.io/citadelplane/citadel-agent@sha256:'+'a'*64),
                        ('copy','ghcr.io/citadel-p/citadel:1.2.3'),
                        ('copy','docker.io/citadelplane/citadel:1.2')]:
            with self.subTest(failure=failure):
                self.setUp(); self.backend.fail=failure
                with self.assertRaises(RuntimeError): self.promote()
                if not failure[1].endswith(':1.2'):
                    self.assertFalse(any(tag in ['1.2','1','latest'] for _,tag in self.backend.images))
                self.record=self.backend.load(self.record['tag'])
                self.assertIsNotNone(self.record)
                self.backend.fail=None
                self.promote()
                self.assertEqual(self.record['status'],'images-verified')

    def test_exact_conflict_and_transport_conversion_stop_promotion(self):
        repo=release.POLICY['registries']['agent'][0]
        self.backend.images[repo,'1.2.3']='sha256:'+'f'*64
        with self.assertRaisesRegex(ValueError,'Immutable'): self.promote()
        self.assertFalse(self.backend.operations)
        self.backend.images={}; self.backend.corrupt_copy=True
        with self.assertRaisesRegex(ValueError,'digest mismatch'): self.promote()
        self.assertFalse(any(op[0]=='sign' for op in self.backend.operations))

    def test_older_retry_never_rolls_back_newer_aliases(self):
        self.promote()
        newer=record('1.2.10'); newer['indexes']={c:'sha256:'+'f'*64 for c in ['agent','core']}
        self.backend.current=newer
        release.promote(self.backend,newer,self.directory)
        self.backend.current=self.record; self.backend.operations=[]
        self.promote()
        self.assertFalse(any(op[0]=='copy' for op in self.backend.operations))
        for repo in release.POLICY['registries']['core']:
            self.assertEqual(self.backend.images[repo,'latest'],'sha256:'+'f'*64)
        self.assertEqual(release.aliases('1.2.3','core',['1.3.0','2.0.0']),['1.2'])

    def test_no_writes_in_dry_run_and_retention_gate(self):
        self.backend.apply=False
        with patch.object(release.subprocess,'run', side_effect=AssertionError('No external command allowed')):
            self.assertTrue(self.promote()['dryRun'])
        self.assertFalse(self.backend.operations)
        real=release.Backend('Citadel-P/Citadel')
        for args in [('git','tag'),('gh','release'),('skopeo','copy'),('cosign','sign')]:
            with self.assertRaisesRegex(RuntimeError,'dry run'): real.command(*args,write=True)
        self.backend.apply=True; self.record['steps']={}
        with self.assertRaisesRegex(ValueError,'retained'): self.promote()
        self.assertFalse(self.backend.operations)


class ReleaseLookupTests(unittest.TestCase):
    def setUp(self):
        self.backend = release.Backend('Citadel-P/Citadel')

    def test_draft_lookup_uses_graphql_id_and_preserves_rest_assets(self):
        draft = dict(id=42, tag_name='v1.2.3', draft=True,
                     assets=[dict(id=91, name='release-record-000001.json')])
        lookup = dict(data=dict(repository=dict(release=dict(databaseId=42))))
        with patch.object(self.backend, 'command', side_effect=[json.dumps(lookup), json.dumps(draft)]) as command:
            self.assertEqual(self.backend.release('v1.2.3'), draft)
        self.assertEqual(command.call_args_list[0].args[:3], ('gh', 'api', 'graphql'))
        self.assertIn('tag=v1.2.3', command.call_args_list[0].args)
        self.assertEqual(command.call_args_list[1].args,
                         ('gh', 'api', 'repos/Citadel-P/Citadel/releases/42'))

    def test_absent_release_returns_none_without_rest_request(self):
        lookup = dict(data=dict(repository=dict(release=None)))
        with patch.object(self.backend, 'command', return_value=json.dumps(lookup)) as command:
            self.assertIsNone(self.backend.release('v1.2.3'))
            command.assert_called_once()

    def test_lookup_errors_are_not_treated_as_absent_releases(self):
        with patch.object(self.backend, 'command', return_value=json.dumps({'errors': [{'message': 'denied'}]})):
            with self.assertRaisesRegex(RuntimeError, 'release lookup failed'):
                self.backend.release('v1.2.3')
        with patch.object(self.backend, 'command', side_effect=RuntimeError('network failure')):
            with self.assertRaisesRegex(RuntimeError, 'network failure'):
                self.backend.release('v1.2.3')

    def test_invalid_release_id_is_rejected(self):
        for release_id in [None, True, 0, -1, '42']:
            with self.subTest(release_id=release_id):
                lookup = dict(data=dict(repository=dict(release=dict(databaseId=release_id))))
                with patch.object(self.backend, 'command', return_value=json.dumps(lookup)):
                    with self.assertRaisesRegex(ValueError, 'Invalid GitHub release ID'):
                        self.backend.release('v1.2.3')

    def test_missing_journal_stops_retention_before_writes(self):
        backend = FakeBackend()
        with self.assertRaisesRegex(ValueError, 'Release journal is not visible'):
            release.retain(backend, record(), Path('.'))
        self.assertEqual(backend.operations, [])


class CommandBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        self.file=self.root/'version.json'; self.file.write_text(json.dumps(metadata()))
        self.backend=FakeBackend(apply=False)
        self.value=record(); self.value['status']='images-verified'
        self.backend.records[self.value['tag']]=self.value
        self.env=dict(CITADEL_RUST_AGENT_RELEASE_ENABLED='true', DOCKERHUB_NAMESPACE='citadelplane',
                      DOCKERHUB_USERNAME='fixture',DOCKERHUB_TOKEN='fixture',DOCS_SITE_URL='https://docs.example.invalid')

    def main(self, command, apply=False):
        args=['release.py',command,'--metadata',str(self.file),'--directory',str(self.root),
              '--repository','Citadel-P/Citadel']+(['--apply'] if apply else [])
        with patch.object(release.sys,'argv',args), patch.object(release,'Backend',return_value=self.backend):
            release.main()

    def test_manual_context_cannot_apply_even_with_release_metadata(self):
        with patch.dict(release.os.environ,dict(self.env,GITHUB_EVENT_NAME='workflow_dispatch',
                        GITHUB_REF='refs/tags/v1.2.3',GITHUB_SHA='a'*40),clear=True):
            for command in ['promote','finalize']:
                with self.assertRaisesRegex(ValueError,'manual runs are read-only'): self.main(command,apply=True)
        self.assertFalse(self.backend.operations)

    def test_documentation_dry_run_no_release_or_docs_writes(self):
        with patch.dict(release.os.environ,self.env,clear=True), patch('builtins.print') as output:
            self.main('docs'); self.main('finalize')
            self.assertEqual(output.call_args_list[0].args[0],'eligible=true')
            self.backend.images['ghcr.io/citadel-p/citadel','2.0.0']='sha256:'+'f'*64
            self.main('docs')
            self.assertEqual(output.call_args_list[-1].args[0],'eligible=false')
        self.assertFalse(self.backend.operations)

    def test_missing_doc_success_cannot_finalize_current_release(self):
        self.backend.apply=True
        with patch.dict(release.os.environ,dict(self.env,GITHUB_EVENT_NAME='push',
                        GITHUB_REF='refs/tags/v1.2.3',GITHUB_SHA='a'*40),clear=True), \
             patch.object(release.version,'git',side_effect=['tag','a'*40]):
            with self.assertRaisesRegex(ValueError,'successful documentation'): self.main('finalize',apply=True)
        self.assertFalse(self.backend.operations)


class PreflightTests(unittest.TestCase):
    def setUp(self):
        self.backend=FakeBackend()
        self.env=dict(CITADEL_RUST_AGENT_RELEASE_ENABLED='true', DOCKERHUB_NAMESPACE='citadelplane',
                      DOCKERHUB_USERNAME='fixture',DOCKERHUB_TOKEN='fixture',DOCS_SITE_URL='https://docs.example.invalid')

    def check(self): return release.preflight(metadata(),self.backend,self.env)

    def test_required_policy_and_bootstrap(self):
        with self.assertRaisesRegex(ValueError,'baselines'): self.check()
        self.env['CITADEL_FIRST_RUST_RELEASE']='1.2.3'
        self.assertTrue(self.check()['bootstrap'])
        for key in ['CITADEL_RUST_AGENT_RELEASE_ENABLED','DOCKERHUB_TOKEN','DOCKERHUB_NAMESPACE','DOCS_SITE_URL']:
            value=self.env.pop(key)
            with self.assertRaises(ValueError): self.check()
            self.env[key]=value
        self.backend.images['ghcr.io/citadel-p/citadel','1.2.2']='sha256:'+'e'*64
        with self.assertRaisesRegex(ValueError,'Bootstrap forbidden'): self.check()

    def test_released_baselines_and_architectures(self):
        prev=record('1.2.2'); prev['status']='complete'; self.backend.records[prev['tag']]=prev
        self.env['CITADEL_AGENT_ROLLBACK_IMAGE']='ghcr.io/citadel-p/citadel.agent@'+prev['indexes']['agent']
        self.env['CITADEL_CORE_ROLLBACK_IMAGE']='ghcr.io/citadel-p/citadel@'+prev['indexes']['core']
        self.assertFalse(self.check()['bootstrap'])
        with patch.object(self.backend,'manifest', return_value=b'{"manifests":[]}'):
            with self.assertRaisesRegex(ValueError,'architecture'): self.check()
        self.env['CITADEL_CORE_ROLLBACK_IMAGE']='ghcr.io/citadel-p/citadel@sha256:'+'c'*64
        with self.assertRaisesRegex(ValueError,'earlier completed'): self.check()

    def test_retry_uses_recorded_baselines_after_newer_release(self):
        original=record(); self.backend.records[original['tag']]=original
        newer=record('1.2.4'); newer['status']='complete'; self.backend.records[newer['tag']]=newer
        self.assertEqual(self.check(),original['compatibility'])
        original['metadata']['sourceRevision']='b'*40
        with self.assertRaisesRegex(ValueError,'identity'): self.check()


class ArtifactTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)

    def test_oci_assembly_deterministic_architectures_and_integrity(self):
        for arch in ['amd64','arm64']: fixture_oci(self.root/f'agent-{arch}.oci.tar',arch,nested=True)
        expected=release.assemble('agent',self.root); archive=release.version.sha256(self.root/'agent.oci.tar')
        self.assertEqual(release.assemble('agent',self.root),expected)
        self.assertEqual(release.version.sha256(self.root/'agent.oci.tar'),archive)
        fixture_oci(self.root/'agent-arm64.oci.tar','amd64')
        with self.assertRaisesRegex(ValueError,'architecture'): release.assemble('agent',self.root)
        (self.root/'agent-arm64.oci.tar').unlink()
        with self.assertRaises(FileNotFoundError): release.assemble('agent',self.root)

    def test_interrupted_upload_recovers_only_original_hashes_and_retains(self):
        backend=FakeBackend(); value=record(); value['steps']={}
        value['artifacts']={'docs.tar':release.digest(b'original').split(':')[1]}
        backend.records[value['tag']]=value; backend.originals['docs.tar']=b'original'
        recovered=release.recover(backend,value['tag'],self.root)
        self.assertFalse(backend.operations)
        release.retain(backend,recovered,self.root)
        self.assertTrue(recovered['steps']['artifacts-retained'])
        backend.assets[value['tag'],'docs.tar']=b'changed'
        with self.assertRaisesRegex(ValueError,'identity lost'): release.recover(backend,value['tag'],self.root)
        del backend.assets[value['tag'],'docs.tar']
        with self.assertRaisesRegex(ValueError,'missing'): release.recover(backend,value['tag'],self.root)


    def test_prepublication_retry_reuses_artifacts_or_stops_if_lost(self):
        backend=release.Backend('Citadel-P/Citadel')
        env=dict(GITHUB_RUN_ID='42',GITHUB_RUN_ATTEMPT='1')
        with patch.dict(release.os.environ,env,clear=True), patch.object(backend,'command',return_value='[{"artifacts":[]}]'):
            self.assertFalse(backend.recover_validation_artifact('candidates-amd64',self.root,metadata()))
        env['GITHUB_RUN_ATTEMPT']='2'
        prior=[{'jobs':[{'name':'candidates / Tested Core and Agent linux/amd64','steps':[
            {'name':'Exercise exact candidates and supported upgrade directions','conclusion':'success'}]}]}]
        with patch.dict(release.os.environ,env,clear=True), patch.object(backend,'command',side_effect=['[{"artifacts":[]}]',json.dumps(prior)]):
            with self.assertRaisesRegex(ValueError,'unavailable'):
                backend.recover_validation_artifact('candidates-amd64',self.root,metadata())
        def command(*args):
            if args[1]=='api': return '[{"artifacts":[{"name":"release-documentation","expired":false}]}]'
            target=Path(args[-1]); (target/'docs.tar').write_bytes(b'original')
            (target/'documentation-inputs.json').write_text(json.dumps(dict(metadata=metadata(),
                artifacts={'docs.tar':release.digest(b'original').split(':')[1]})))
            return ''
        with patch.dict(release.os.environ,env,clear=True), patch.object(backend,'command',side_effect=command):
            self.assertTrue(backend.recover_validation_artifact('release-documentation',self.root,metadata()))
            self.assertEqual((self.root/'docs.tar').read_bytes(),b'original')
            with self.assertRaisesRegex(ValueError,'metadata differs'):
                backend.recover_validation_artifact('release-documentation',self.root,metadata('1.2.4'))

    def test_missing_or_failed_acceptance_and_tampering_block(self):
        value=record()
        for arch in ['amd64','arm64']:
            for component in ['core','agent']:
                (self.root/f'{component}-{arch}.oci.tar').write_bytes(b'candidate')
            evidence=dict(metadata=value['metadata'], compatibility=value['compatibility'],passed=True,
                          architecture=arch, checks=['independent-binary-and-oci-versions','core-runtime',
                          'agent-image','agent-compatibility','candidate-acceptance'],mixedVersionChecks=[],
                          artifacts={c:release.digest(b'candidate').split(':')[1] for c in ['core','agent']})
            (self.root/f'evidence-{arch}.json').write_text(json.dumps(evidence))
        release.validate_evidence(self.root,value['metadata'],value['compatibility'])
        file=self.root/'evidence-arm64.json'; evidence['passed']=False; file.write_text(json.dumps(evidence))
        with self.assertRaisesRegex(ValueError,'acceptance'): release.validate_evidence(self.root,value['metadata'],value['compatibility'])
        evidence['passed']=True; evidence['checks'].remove('core-runtime'); file.write_text(json.dumps(evidence))
        with self.assertRaisesRegex(ValueError,'checks'): release.validate_evidence(self.root,value['metadata'],value['compatibility'])
        file.unlink()
        with self.assertRaises(FileNotFoundError): release.validate_evidence(self.root,value['metadata'],value['compatibility'])


if __name__=='__main__': unittest.main()
