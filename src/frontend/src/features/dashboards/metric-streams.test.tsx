import { act, screen, waitFor } from '@testing-library/react';
import type { StackServiceHistory } from '@/api/generated/api.types';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { useStackSeries } from './stack-series';
import { useWorkloadSeries } from './workload-series';

function Stack({ id, member, value }: { id: string; member: string; value: number }) {
  const history = [
    { serviceName: 'web', containerIds: [member], stats: [{ created: 10, cpuUsage: value }] },
  ] as StackServiceHistory[];
  const [series] = useStackSeries(id, history, 10, 3600);
  return <output>{series.stats.map((s) => s.cpuUsage).join(',')}</output>;
}
it('keeps the Stack subscription through history and membership changes and releases it on source change', async () => {
  const live = new FakeRealtimeConnection();
  const { rerender } = renderCitadel(<Stack id="a" member="first" value={1} />, {
    groups: { connectionFactory: () => live },
  });
  const calls = () => live.invoke.mock.calls.filter(([, group]) => String(group).startsWith('stack-info:'));
  const emit = (id: string, created: number, cpuUsage: number, stack = 'a') =>
    act(() => live.emit('ReceiveStackContainerStats', [{ id, containerStat: { created, cpuUsage } }], stack));
  await waitFor(() => expect(calls()).toEqual([['JoinGroup', 'stack-info:a']]));
  rerender(<Stack id="a" member="first" value={2} />);
  emit('first', 30, 3);
  expect(screen.getByRole('status')).toHaveTextContent('2,3');
  rerender(<Stack id="a" member="second" value={4} />);
  emit('first', 40, 5);
  emit('second', 40, 6);
  expect(screen.getByRole('status').textContent).toBe('4,6');
  expect(calls()).toEqual([['JoinGroup', 'stack-info:a']]);
  rerender(<Stack id="b" member="second" value={7} />);
  await waitFor(() => expect(calls()).toContainEqual(['LeaveGroup', 'stack-info:a']));
  emit('second', 50, 100);
  expect(screen.getByRole('status').textContent).toBe('7');
  emit('second', 50, 8, 'b');
  expect(screen.getByRole('status').textContent).toBe('7,8');
  rerender(<></>);
  await waitFor(() => expect(calls()).toContainEqual(['LeaveGroup', 'stack-info:b']));
});

function Service() {
  const { stats } = useWorkloadSeries('service-stats:one', 10, 3600);
  return <output>{JSON.stringify(stats.map((s) => [s.created, s.cpuUsage, s.rxBytesPerSecond]))}</output>;
}
it('aggregates complete replica buckets, derives rates before summing, and resets on replacement', async () => {
  const live = new FakeRealtimeConnection();
  renderCitadel(<Service />, { groups: { connectionFactory: () => live } });
  await waitFor(() => expect(live.invoke).toHaveBeenCalledWith('JoinGroup', 'service-stats:one'));
  const members = (...ids: string[]) =>
    act(() => live.emit('WorkloadStatsMembership', { group: 'service-stats:one', members: ids, expected: ids.length }));
  const emit = (containerId: string, created: number, rxBytes: number, cpuUsage: number) =>
    act(() =>
      live.emit('WorkloadStatsUpdated', {
        group: 'service-stats:one',
        stats: [{ containerId, created, rxBytes, cpuUsage }],
      }),
    );
  members('a', 'b');
  emit('other', 100, 99, 99);
  emit('a', 100, 100, 1);
  expect(screen.getByRole('status').textContent).toBe('[]');
  emit('b', 100, 50, 2);
  emit('a', 110, 10, 3);
  expect(screen.getByRole('status').textContent).toBe('[[100,3,null]]');
  emit('b', 110, 80, 4);
  expect(screen.getByRole('status').textContent).toBe('[[100,3,null],[110,7,4]]');
  members('new');
  emit('a', 120, 100, 42);
  emit('new', 120, 100, 8);
  expect(screen.getByRole('status').textContent).toBe('[[120,8,null]]');
});
