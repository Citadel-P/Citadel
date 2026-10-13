import { describe, expect, it } from 'vitest';
import { BackupSourceSpec } from '@/api/generated/api.types';
import { backupConfigurationUrl, backupReturnPath, backupSourceFromSearch } from './navigation';

describe('contextual backup navigation', () => {
  it.each<BackupSourceSpec>([
    { $type: 'Stack', stackId: 'stack' },
    { $type: 'SwarmService', swarmServiceId: 'service' },
    { $type: 'Deployment', deploymentId: 'deployment' },
    { $type: 'DockerVolume', platformId: 'platform', volumeName: 'db & uploads', dockerNodeId: 'node/1' },
  ])('prefills $type sources without losing volume or node identity', (source) => {
    const query = new URL(backupConfigurationUrl(source, '/stacks/edit/stack#config'), 'http://localhost').searchParams;
    expect(backupSourceFromSearch(query)).toMatchObject(source);
    expect(backupReturnPath(query)).toBe('/stacks/edit/stack#config');
  });
  it('returns to workload pages with a trailing slash, including Swarm Services', () => {
    for (const returnTo of ['/stacks/edit/stack/', '/deployments/edit/app/#config', '/swarm-services/edit/service/']) {
      expect(backupReturnPath(new URLSearchParams({ returnTo }))).toBe(returnTo);
    }
  });
  it('rejects incomplete and unsupported source selections', () => {
    expect(backupSourceFromSearch(new URLSearchParams('sourceType=DockerVolume&platformId=p'))).toBeUndefined();
    expect(backupSourceFromSearch(new URLSearchParams('sourceType=unsupported'))).toBeUndefined();
  });
  it.each(['https://evil.example', '//evil.example', '/\\evil.example', '/backup-policies', '/stacks/edit/ok\n'])(
    'rejects unsafe or unrelated return destinations: %s',
    (returnTo) => {
      expect(backupReturnPath(new URLSearchParams({ returnTo }))).toBeUndefined();
    },
  );
});
