import { StackReleaseStatus } from '@/api/generated/api.types';
import { getServiceAvailability } from '../shared';

describe('getServiceAvailability', () => {
  it.each([
    [{ runningTaskCount: 2, desiredTaskCount: 2 }, StackReleaseStatus.Healthy],
    [{ runningTaskCount: 1, desiredTaskCount: 2 }, StackReleaseStatus.Degraded],
    [{ runningTaskCount: 0, desiredTaskCount: 2 }, StackReleaseStatus.Failed],
    [{ runningTaskCount: 0, desiredTaskCount: 0 }, StackReleaseStatus.Stopped],
  ])('maps replica availability to the service indicator', (counts, expected) => {
    expect(getServiceAvailability(service(counts)).status).toBe(expected);
  });

  it('reports current availability independently from a previous paused update', () => {
    expect(
      getServiceAvailability(service({ runningTaskCount: 2, desiredTaskCount: 2, updateState: 'Paused' })).status,
    ).toBe(StackReleaseStatus.Healthy);
  });

  it('keeps a paused update degraded while replicas are unavailable', () => {
    expect(
      getServiceAvailability(service({ runningTaskCount: 1, desiredTaskCount: 2, updateState: 'Paused' })).status,
    ).toBe(StackReleaseStatus.Degraded);
  });

  it('does not report stale inventory as current health', () => {
    expect(getServiceAvailability(service({ runningTaskCount: 2, desiredTaskCount: 2, isStale: true })).status).toBe(
      StackReleaseStatus.Unknown,
    );
  });
});

const service = (overrides: Partial<ServiceAvailabilityInput> = {}): ServiceAvailabilityInput => ({
  runningTaskCount: 1,
  desiredTaskCount: 1,
  updateState: 'Completed',
  isStale: false,
  ...overrides,
});

type ServiceAvailabilityInput = Parameters<typeof getServiceAvailability>[0];
