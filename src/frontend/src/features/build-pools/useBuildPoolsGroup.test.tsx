import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useBuildPoolsGroup } from './useBuildPoolsGroup';

const readyPool = {
  id: 'pool-1',
  name: 'Edge builder',
  lastValidationStatus: 'Ready',
  tags: [{ id: 'tag-1', name: 'production' }],
};

function Probe() {
  const { pools } = useBuildPoolsGroup();
  return <output>{pools.map((pool) => `${pool.name}: ${pool.lastValidationStatus}`).join(', ')}</output>;
}

afterEach(() => vi.useRealTimers());

it('applies realtime health changes without another request or a page reload', async () => {
  const read = vi.fn(() => HttpResponse.json({ pools: [readyPool], capabilities: {} }));
  server.use(http.get('*/api/v1/buildAgentPools', read));
  const connection = new FakeRealtimeConnection();
  renderCitadel(<Probe />, {
    groups: {
      connectionFactory: () => connection,
      startConnection: (hub) => hub.start(),
    },
  });
  expect(await screen.findByText('Edge builder: Ready')).toBeVisible();
  await waitFor(() => expect(connection.invoke).toHaveBeenCalledWith('JoinGroup', 'build-agent-pools'));

  act(() => connection.emit('BuildAgentPoolInfoUpdated', { ...readyPool, lastValidationStatus: 'Invalid' }, 'update'));
  expect(await screen.findByText('Edge builder: Invalid')).toBeVisible();
  expect(read).toHaveBeenCalledOnce();
});

it('does not poll and recovers missed health updates on reconnect, preserving filters', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  let pool = readyPool;
  const read = vi.fn(({ request }: { request: Request }) => {
    expect(new URL(request.url).searchParams.getAll('tags')).toEqual(['production']);
    return HttpResponse.json({ pools: [pool], capabilities: {} });
  });
  server.use(http.get('*/api/v1/buildAgentPools', read));
  const connection = new FakeRealtimeConnection();
  renderCitadel(<Probe />, {
    route: '/build-pools?tags=production',
    groups: {
      connectionFactory: () => connection,
      startConnection: (hub) => hub.start(),
    },
  });
  expect(await screen.findByText('Edge builder: Ready')).toBeVisible();
  await waitFor(() => expect(connection.invoke).toHaveBeenCalledWith('JoinGroup', 'build-agent-pools'));

  await act(() => vi.advanceTimersByTimeAsync(90_000));
  expect(read).toHaveBeenCalledOnce();

  // A connection interruption misses an update; reconnect reconciles the list.
  pool = { ...readyPool, lastValidationStatus: 'Invalid' };
  act(() => {
    connection.reconnecting();
    connection.reconnected();
  });
  expect(await screen.findByText('Edge builder: Invalid')).toBeVisible();
  expect(read).toHaveBeenCalledTimes(2);
});

it('updates Edge connection status independently of the last capability check', async () => {
  server.use(
    http.get('*/api/v1/buildAgentPools', () =>
      HttpResponse.json({
        pools: [{ ...readyPool, connectionStatus: 'Connected' }],
        capabilities: {},
      }),
    ),
  );
  const connection = new FakeRealtimeConnection();
  function ConnectionProbe() {
    const { pools } = useBuildPoolsGroup();
    return (
      <output>
        {pools[0]?.connectionStatus} / {pools[0]?.lastValidationStatus}
      </output>
    );
  }
  renderCitadel(<ConnectionProbe />, {
    groups: { connectionFactory: () => connection, startConnection: (hub) => hub.start() },
  });
  expect(await screen.findByText('Connected / Ready')).toBeVisible();
  await waitFor(() => expect(connection.invoke).toHaveBeenCalledWith('JoinGroup', 'build-agent-pools'));
  act(() => connection.emit('BuildAgentPoolInfoUpdated', { ...readyPool, connectionStatus: 'Offline' }, 'update'));
  expect(await screen.findByText('Offline / Ready')).toBeVisible();
});
