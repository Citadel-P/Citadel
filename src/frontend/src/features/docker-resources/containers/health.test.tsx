import { describe, expect, it } from 'vitest';
import { ContainerHealthStatus as Status, ContainerStateStatus, type ContainerView } from '@/api/generated/api.types';
import { currentHealth } from './health';
import { applyContainerStatePatches } from './hooks/container-order';

const now = 2_000_000;
const health = { status: Status.Healthy, observedAt: now, lastCheckAt: null, failingStreak: 0, reason: 'Passing' };
describe('container health evidence', () => {
  it('never treats missing, stale or disconnected evidence as healthy', () => {
    expect(currentHealth({ state: 'Running' }, now)).toBe(Status.Unknown);
    expect(currentHealth({ state: 'Running', health }, now + 180_001)).toBe(Status.Unknown);
    expect(currentHealth({ state: 'Running', health, projectionStaleSince: 1 }, now)).toBe(Status.Unknown);
    expect(currentHealth({ state: 'Offline', health }, now)).toBe(Status.Unknown);
    expect(currentHealth({ state: 'Running', health }, now)).toBe(Status.Healthy);
  });
  it('separates inactive, no check, disabled and failed checks', () => {
    expect(currentHealth({ state: 'Paused', health }, now)).toBe(Status.NotRunning);
    expect(currentHealth({ state: 'Exited', health }, now)).toBe(Status.NotRunning);
    for (const status of [Status.NoCheck, Status.Disabled, Status.Unhealthy, Status.Starting]) {
      expect(currentHealth({ state: 'Running', health: { ...health, status } }, now)).toBe(status);
    }
  });
  it('invalidates health on a lifecycle patch even if the observed state is still Running', () => {
    const container = {
      id: 'persisted',
      containerId: 'docker-id',
      state: ContainerStateStatus.Running,
      health,
    } as ContainerView;
    const result = applyContainerStatePatches(
      [container],
      [{ id: 'persisted', containerId: 'docker-id', state: ContainerStateStatus.Running }],
    );
    expect(result[0].health).toBeNull();
    expect(currentHealth(result[0], now)).toBe(Status.Unknown);
    expect(
      applyContainerStatePatches([container], [{ id: 'persisted', containerId: 'docker-id', updated: 42 }])[0].health,
    ).toEqual(health);
  });
});
