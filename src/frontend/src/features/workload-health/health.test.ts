import { describe, expect, it } from 'vitest';
import { ContainerHealthStatus as Status, PlatformStatus, WorkloadHealthStatus } from '@/api/generated/api.types';
import { createWorkloadHealth } from '@/test/factories/resources';
import { healthOrder, matchesHealth, workloadHealthStatus } from './health';
const now = 1_000_000;
const resource = () => ({
  id: 'owner',
  name: 'api',
  platformStatus: PlatformStatus.Online,
  probeHealth: createWorkloadHealth({
    status: WorkloadHealthStatus.Partial,
    evaluatedAt: now,
    observedAt: now,
    observed: 3,
    expected: 3,
    missing: 0,
    counts: { healthy: 1, unhealthy: 0, starting: 0, noCheck: 1, disabled: 0, notRunning: 0, unknown: 1, completed: 0 },
  }),
});
describe('workload health evidence', () => {
  it('matches mixed coverage without treating one passing member as overall Healthy', () => {
    const value = resource();
    expect(matchesHealth(value, ['Healthy'], now)).toBe(false);
    expect(matchesHealth(value, ['NoCheck'], now)).toBe(true);
    expect(matchesHealth(value, ['Unknown'], now)).toBe(true);
    expect(matchesHealth(value, ['Partial'], now)).toBe(true);
  });
  it('keeps failures filterable even when the bounded member breakdown omits members', () => {
    const value = resource();
    value.probeHealth.counts.unhealthy = 1;
    value.probeHealth.membersTruncated = true;
    expect(matchesHealth(value, ['Unhealthy'], now)).toBe(true);
  });
  it('expires old or future evidence and treats offline platforms as Unknown', () => {
    for (const time of [now - 1, now + 180_001]) {
      expect(workloadHealthStatus(resource(), time)).toBe(Status.Unknown);
      expect(matchesHealth(resource(), ['NoCheck'], time)).toBe(false);
    }
    expect(workloadHealthStatus({ ...resource(), platformStatus: PlatformStatus.Offline }, now)).toBe(Status.Unknown);
    expect(matchesHealth(resource(), ['NoCheck'], now, false)).toBe(false);
    expect(matchesHealth(resource(), ['Unknown'], now, false)).toBe(true);
  });
  it('sorts by operational relevance rather than alphabetically', () => {
    expect(healthOrder.indexOf(Status.Unhealthy)).toBeLessThan(healthOrder.indexOf(Status.Healthy));
    expect(healthOrder.indexOf(Status.Unknown)).toBeLessThan(healthOrder.indexOf(Status.Starting));
  });
  it('preserves a fresh failure when a sibling expires, including truncated breakdowns', () => {
    const value = resource();
    value.probeHealth.status = WorkloadHealthStatus.Unhealthy;
    value.probeHealth.observedAt = now - 170_000;
    value.probeHealth.validUntil = now + 10_000;
    value.probeHealth.unhealthyUntil = now + 180_000;
    value.probeHealth.counts.unhealthy = 1;
    value.probeHealth.membersTruncated = true;
    expect(workloadHealthStatus(value, now + 30_000)).toBe(Status.Unhealthy);
    expect(matchesHealth(value, ['Unhealthy'], now + 30_000)).toBe(true);
    expect(matchesHealth(value, ['Unknown'], now + 30_000)).toBe(true);
    expect(matchesHealth(value, ['NoCheck'], now + 30_000)).toBe(false);
    expect(workloadHealthStatus(value, now + 180_001)).toBe(Status.Unknown);
  });
  it('does not extend Healthy coverage when one required observation expires', () => {
    const value = resource();
    value.probeHealth.status = WorkloadHealthStatus.Healthy;
    value.probeHealth.validUntil = now + 10_000;
    expect(workloadHealthStatus(value, now + 30_000)).toBe(Status.Unknown);
    expect(matchesHealth(value, ['Healthy'], now + 30_000)).toBe(false);
  });
  it('stops matching an expired failure while a starting probe remains fresh', () => {
    const value = resource();
    value.probeHealth.status = WorkloadHealthStatus.Unhealthy;
    value.probeHealth.validUntil = now + 10_000;
    value.probeHealth.unhealthyUntil = now + 10_000;
    value.probeHealth.startingUntil = now + 180_000;
    value.probeHealth.counts.unhealthy = 1;
    expect(workloadHealthStatus(value, now + 30_000)).toBe(Status.Starting);
    expect(matchesHealth(value, ['Unhealthy'], now + 30_000)).toBe(false);
    expect(matchesHealth(value, ['Starting'], now + 30_000)).toBe(true);
  });
});
