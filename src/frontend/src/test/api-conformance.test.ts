import fs from 'node:fs';
import { clientSignatures } from '../../scripts/client-signatures';
import { resources, scriptResourceKeys } from '@/api/generated/resources';
import { Api } from '@/api/generated/api.types';

it('metadata matches every generated method in its real namespace', () => {
  const signatures = clientSignatures(fs.readFileSync('src/api/generated/api.types.ts', 'utf-8'));
  const client = new Api();
  expect(Object.keys(resources).sort()).toEqual([...signatures.keys()].sort());
  for (const [name, resource] of Object.entries(resources)) {
    const signature = signatures.get(name)!;
    expect(resource.namespace).toBe(signature.namespace);
    expect(resource.params).toEqual(signature.params);
    expect(resource.requiredParams).toEqual(signature.requiredParams);
    expect((client as any)[signature.namespace][name]).toBeTypeOf('function');
  }
  expect(new Set([...signatures.values()].map((signature) => signature.namespace))).toEqual(
    new Set(['api', 'health', 'listener', 'metrics', 'ready']),
  );
});

it('keeps administrator installation settings out of the scripts API', () => {
  expect(resources.getInstallationSettings.path).toBe('/api/v1/installation/settings');
  expect(resources.patchInstallationSettings.path).toBe('/api/v1/installation/settings');
  expect(scriptResourceKeys).not.toContain('getInstallationSettings');
  expect(scriptResourceKeys).not.toContain('patchInstallationSettings');
});
