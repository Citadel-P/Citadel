import { SwarmServiceSynchronizationState, type ManagedSwarmServiceView } from '@/api/generated/api.types';
import { normalizeManagedSwarmService } from './useSwarmServiceGroup';

describe('normalizeManagedSwarmService', () => {
  it('normalizes MessagePack image unions and preserves REST-only capabilities', () => {
    const capabilities = { canRead: true, canApply: true };
    const tasks = [{ id: 'task-1' }];
    const current = { capabilities, tasks } as ManagedSwarmServiceView;
    const update = {
      id: 'service-id',
      appliedImageDigest: 'sha256:new',
      capabilities: undefined,
      spec: {
        image: ['External', { registryId: 'registry-id', imageTag: 'nginx:latest' }],
      },
    } as unknown as ManagedSwarmServiceView;

    const result = normalizeManagedSwarmService(current, update);

    expect(result.capabilities).toBe(capabilities);
    expect(result.tasks).toBe(tasks);
    expect(result.appliedImageDigest).toBe('sha256:new');
    expect(result.spec.image).toEqual({
      $type: 'External',
      registryId: 'registry-id',
      imageTag: 'nginx:latest',
    });
    expect(Object.keys(result.spec.image)[0]).toBe('$type');
  });

  it('clears stale runtime tasks when Docker reports the Service missing', () => {
    const result = normalizeManagedSwarmService(
      { tasks: [{ id: 'task-1' }] } as ManagedSwarmServiceView,
      {
        synchronizationState: SwarmServiceSynchronizationState.RuntimeMissing,
        tasks: undefined,
        spec: { image: { $type: 'External' } },
      } as unknown as ManagedSwarmServiceView,
    );

    expect(result.tasks).toEqual([]);
  });

  it('preserves the current image when an incomplete realtime union is received', () => {
    const image = {
      $type: 'External',
      registryId: 'registry-id',
      imageTag: 'redis',
    } as const;
    const result = normalizeManagedSwarmService(
      { spec: { image } } as ManagedSwarmServiceView,
      { spec: { image: {} } } as unknown as ManagedSwarmServiceView,
    );

    expect(result.spec.image).toBe(image);
  });
});
