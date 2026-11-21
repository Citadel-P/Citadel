import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { StackBackupCell } from '@/features/stacks/backup-cell';
import { useContainersGroup } from './useContainersGroup';

const platformId = '00000000-0000-0000-0000-000000000201';
function Containers() {
  const { containersInfo } = useContainersGroup(platformId);
  return <output>{JSON.stringify(containersInfo?.stackBackupCoverage)}</output>;
}

it('uses one containers group per browser and replaces badges after reconnect and revocation', async () => {
  const fake = new FakeRealtimeConnection();
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const read = vi.fn(async () => {
    await gate;
    return HttpResponse.json({ containers: [], stackBackupCoverage: { stale: {} } });
  });
  server.use(http.get(`http://localhost/api/v1/platforms/${platformId}/containers`, read));
  renderCitadel(<Containers />, { groups: { connectionFactory: () => fake, startConnection: (c) => c.start() } });
  await waitFor(() => expect(read).toHaveBeenCalledOnce());
  await waitFor(() => expect(fake.listenerCount('ContainersInfoUpdated')).toBe(1));
  const containers = [{ id: 'one', containerId: 'docker-one', stackId: 'stack' }];
  act(() =>
    fake.emit('ContainersInfoUpdated', {
      platformId,
      containers,
      stackBackupCoverage: { stack: { status: 'Protected' } },
    }),
  );
  await act(async () => {
    release();
    await gate;
  });
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Protected'));
  expect(fake.invoke.mock.calls.filter(([method]) => method === 'JoinGroup')).toEqual([
    ['JoinGroup', `containers:${platformId}`],
  ]);
  act(() =>
    fake.emit('ContainersChanged', {
      platformId,
      containerId: 'docker-one',
      containers,
      stackIds: ['stack'],
      stackBackupCoverage: {},
    }),
  );
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('{}'));
  act(() => fake.reconnecting());
  await waitFor(() => expect(fake.listenerCount('ContainersInfoUpdated')).toBe(0));
  act(() => fake.reconnected());
  await waitFor(() => expect(fake.listenerCount('ContainersInfoUpdated')).toBe(1));
  act(() => fake.emit('ContainersInfoUpdated', { platformId, containers: [], stackBackupCoverage: {} }));
  expect(screen.getByRole('status')).toHaveTextContent('{}');
  // The shared reconnect handler revalidates active HTTP queries once per page.
  expect(read).toHaveBeenCalledTimes(2);
});

it.each([0, 1, 3])('opens one group per containers page with 30 managed Stack badges (%i browsers)', async (count) => {
  const stackIds = Array.from({ length: 30 }, (_, i) => `stack-${i}`);
  const payload = { containers: stackIds.map((id) => ({ id, stackId: id, containerId: id })), stackBackupCoverage: {} };
  server.use(http.get(`http://localhost/api/v1/platforms/${platformId}/containers`, () => HttpResponse.json(payload)));
  function Badges() {
    const { containersInfo } = useContainersGroup(platformId);
    return containersInfo?.containers.map(
      (c) =>
        c.stackId && (
          <StackBackupCell key={c.id} stackId={c.stackId} coverage={containersInfo.stackBackupCoverage[c.stackId]} />
        ),
    );
  }
  const sockets = Array.from({ length: count }, () => new FakeRealtimeConnection());
  for (const fake of sockets)
    renderCitadel(<Badges />, { groups: { connectionFactory: () => fake, startConnection: (c) => c.start() } });
  await waitFor(() =>
    expect(screen.queryAllByRole('button', { name: 'Backups: Coverage unavailable' })).toHaveLength(count * 30),
  );
  const joined = sockets.flatMap((fake) =>
    fake.invoke.mock.calls.filter(([method]) => method === 'JoinGroup').map(([, name]) => name),
  );
  expect(joined).toEqual(Array.from({ length: count }, () => `containers:${platformId}`));
});
