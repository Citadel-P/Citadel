import { ContainerHealthStatus as Status, ContainerStateStatus, type ContainerView } from '@/api/generated/api.types';
import { groupHealth, withContainerGroupHealth } from './group-health';
import { buildContainerRows } from './table';
import { isContainerStackGroup } from './actions';

vi.hoisted(() => {
  document.queryCommandSupported ??= () => false;
});
const now = 2_000_000;
const container = (id: string, status: Status, platformId = 'host'): ContainerView =>
  ({
    id,
    platformId,
    name: id,
    containerId: id,
    stack: 'wordpress',
    state: ContainerStateStatus.Running,
    health: { status, observedAt: now, reason: '' },
  }) as ContainerView;

it.each([
  [[Status.Healthy, Status.Healthy], Status.Healthy],
  [[Status.Healthy, Status.Unhealthy, Status.Unknown], Status.Unhealthy],
  [[Status.Starting, Status.Unhealthy], Status.Unhealthy],
  [[Status.Healthy, Status.Starting, Status.NoCheck], Status.Starting],
  [[Status.Healthy, Status.NoCheck], 'Partial'],
  [[Status.Healthy, Status.Disabled], 'Partial'],
  [[Status.Healthy, Status.Unknown], 'Partial'],
  [[Status.NoCheck, Status.Disabled], Status.NoCheck],
  [[Status.Unknown, Status.Unknown], Status.Unknown],
  [[], Status.Unknown],
] as const)('aggregates %j as %s', (states, expected) => {
  const summary = groupHealth(
    states.map((status, i) => container(String(i), status)),
    now,
  );
  expect(summary.status).toBe(expected);
  expect(Object.values(summary.counts).reduce((sum, count) => sum + count, 0)).toBe(states.length);
});

it('never treats stale, future or inactive observations as passing checks', () => {
  const healthy = container('web', Status.Healthy);
  const stale = { ...container('db', Status.Healthy), projectionStaleSince: now };
  expect(groupHealth([healthy, stale], now).status).toBe('Partial');
  expect(groupHealth([healthy], now + 180_001).status).toBe(Status.Unknown);
  expect(groupHealth([healthy], now - 1).status).toBe(Status.Unknown);
  const stopped = { ...healthy, state: ContainerStateStatus.Exited };
  expect(groupHealth([stopped], now).status).toBe(Status.NotRunning);
  expect(groupHealth([stopped, healthy], now).status).toBe('Partial');
});

it('keeps health independent of filtered group membership without broadening selections', () => {
  const rows = withContainerGroupHealth([container('web', Status.Healthy), container('db', Status.Unhealthy)], now);
  const visible = buildContainerRows(
    rows.filter((row) => row.name === 'web'),
    { cpuCount: 1, memoryTotal: 1 },
  );
  const group = visible[0];
  expect(isContainerStackGroup(group)).toBe(true);
  if (!isContainerStackGroup(group)) throw new Error('Expected stack group');
  expect(group.healthSummary?.status).toBe(Status.Unhealthy);
  expect(group.healthSummary?.counts).toMatchObject({ Healthy: 1, Unhealthy: 1 });
  expect(group.healthSummary?.members?.map((member) => member.containerId)).toEqual(['web', 'db']);
  expect(group.containers.map((row) => row.id)).toEqual(['web']);
});

it('isolates groups by platform and stack name and follows inventory changes', () => {
  const rows = [
    container('web', Status.Healthy),
    container('db', Status.Unhealthy, 'other-host'),
    { ...container('redis', Status.Unhealthy), stack: 'other-stack' },
  ];
  expect(withContainerGroupHealth(rows, now)[0].stackHealth?.status).toBe(Status.Healthy);
  expect(withContainerGroupHealth(rows, now)[0].stackHealth?.members?.map((member) => member.containerId)).toEqual([
    'web',
  ]);
  expect(withContainerGroupHealth([...rows, container('worker', Status.Unhealthy)], now)[0].stackHealth?.status).toBe(
    Status.Unhealthy,
  );
  expect(withContainerGroupHealth(rows, now + 180_001)[0].stackHealth?.status).toBe(Status.Unknown);
});

it('preserves persisted identity and node scope for container diagnostics', () => {
  const source = {
    ...container('persisted-id', Status.Healthy),
    containerId: 'docker-id',
    dockerNodeId: 'worker-node',
    projectionStaleSince: now,
  };
  const member = withContainerGroupHealth([source], now)[0].stackHealth?.members?.[0];
  expect(member).toMatchObject({
    containerId: 'persisted-id',
    dockerContainerId: 'docker-id',
    platformId: 'host',
    nodeId: 'worker-node',
    available: false,
    status: 'Unknown',
    observation: { observedAt: now },
  });
});
