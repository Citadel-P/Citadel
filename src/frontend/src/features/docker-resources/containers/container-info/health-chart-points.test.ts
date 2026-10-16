import { ContainerHealthStatus as Status, type ContainerHealthTransitionView } from '@/api/generated/api.types';
import { groupHealthChartPoints } from './health-chart-points';

const observation = (observedAt: number, status = Status.Healthy): ContainerHealthTransitionView => ({
  observedAt,
  firstObservedAt: observedAt,
  sampleCount: 1,
  state: 'Running',
  health: { status, reason: '' },
  origin: 'Sample',
});

it('combines nearby observations while preserving counts, actual timestamps, and input records', () => {
  const entries = [observation(100), { ...observation(95), firstObservedAt: 90, sampleCount: 4 }, observation(80)];
  const original = structuredClone(entries);
  const groups = groupHealthChartPoints(entries, 14);
  expect(groups.map(({ entry }) => [entry.observedAt, entry.firstObservedAt, entry.sampleCount])).toEqual([
    [100, 90, 5],
    [80, 80, 1],
  ]);
  expect(groups[0].keys).toContain(JSON.stringify(entries[1]));
  expect(entries).toEqual(original);
});

it('does not merge different health states, lifecycle states, OOM evidence or exit codes', () => {
  const entries = [
    observation(100),
    observation(99, Status.Unhealthy),
    { ...observation(98, Status.NotRunning), state: 'Exited', exitCode: 137, oomKilled: true },
    { ...observation(97, Status.NotRunning), state: 'Exited', exitCode: 1 },
    { ...observation(96, Status.NotRunning), state: 'Paused' },
    { ...observation(95, Status.NotRunning), state: 'Exited', exitCode: 137, oomKilled: false },
  ];
  expect(groupHealthChartPoints(entries, 14)).toHaveLength(entries.length);
});

it('bounds dense groups by their latest point instead of chaining a whole run together', () => {
  const entries = [observation(70), observation(90), observation(80), observation(100)];
  expect(groupHealthChartPoints(entries, 14).map(({ entry }) => entry.observedAt)).toEqual([100, 80]);
  expect(groupHealthChartPoints(entries, 7)).toHaveLength(4);
});
