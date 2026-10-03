import { PlatformConnectorType, PlatformType, PlatformView } from '@/api/generated/api.types';
import { describe, expect, it } from 'vitest';
import { createDefaultPlatformInput, platformToFormInput } from './usePlatformForm';

describe('platform form pruning setting', () => {
  it('enables historical Swarm task container pruning by default', () => {
    expect(createDefaultPlatformInput().pruneHistoricalSwarmTaskContainers).toBe(true);
  });

  it('preserves a disabled setting when editing a platform', () => {
    const platform = {
      id: 'platform-1',
      name: 'Swarm',
      address: 'https://manager:9000',
      description: null,
      type: PlatformType.DockerSwarm,
      connectorType: PlatformConnectorType.Agent,
      pruneHistoricalSwarmTaskContainers: false,
    } as PlatformView;

    expect(platformToFormInput(platform).pruneHistoricalSwarmTaskContainers).toBe(false);
  });
});
