import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import type { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useContainerHealthHistory } from './useContainerHealthHistory';

const realtime = vi.hoisted(() => ({
  options: undefined as Parameters<typeof useRealtimeGroup>[0] | undefined,
  connected: true,
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: (options: Parameters<typeof useRealtimeGroup>[0]) => {
    realtime.options = options;
    return { isLoading: false, isConnected: realtime.connected };
  },
}));
beforeEach(() => {
  realtime.connected = true;
});
function Probe({ id = 'one', enabled = true }: { id?: string; enabled?: boolean }) {
  const history = useContainerHealthHistory(id, enabled);
  return (
    <>
      <pre data-testid="history">{JSON.stringify(history)}</pre>
      <button onClick={() => void history.refetch()}>Retry</button>
    </>
  );
}
function subscribe() {
  const on = vi.fn();
  const off = vi.fn();
  const connection = { on, off } as unknown as RealtimeConnection;
  realtime.options!.setupEventListeners(connection);
  expect(on).toHaveBeenCalledWith('ContainerHealthHistoryChanged', expect.any(Function));
  return { receive: on.mock.calls[0][1] as (notification: { resourceId: string }) => Promise<void>, off, connection };
}
const entries = (status = 'Healthy') => [
  { observedAt: 1234, state: 'Running', origin: 'Inspection', health: { status, reason: 'Recorded change' } },
];
const endpoint = 'http://localhost/api/v1/containers/:id/health-history';
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((complete) => {
    resolve = complete;
  });
  return { promise, resolve };
}

it('loads history once through HTTP even without a realtime connection', async () => {
  realtime.connected = false;
  const requests = vi.fn(() => HttpResponse.json(entries()));
  server.use(http.get(endpoint, requests));
  const { rerender } = renderCitadel(<Probe />);
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Healthy'));
  expect(screen.getByTestId('history')).toHaveTextContent('"isConnected":false');
  expect(screen.getByTestId('history')).toHaveTextContent('"isLoading":false');
  rerender(<Probe />);
  expect(requests).toHaveBeenCalledTimes(1);
});

it('skips reads and subscriptions while disabled or without an ID', async () => {
  const requests = vi.fn(() => HttpResponse.json(entries()));
  server.use(http.get(endpoint, requests));
  const { rerender } = renderCitadel(<Probe enabled={false} />);
  expect(realtime.options?.skip).toBe(true);
  expect(requests).not.toHaveBeenCalled();
  rerender(<Probe id="" />);
  expect(realtime.options?.skip).toBe(true);
  expect(requests).not.toHaveBeenCalled();
  rerender(<Probe />);
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Healthy'));
  expect(realtime.options?.skip).toBe(false);
  rerender(<Probe enabled={false} />);
  expect(realtime.options?.skip).toBe(true);
  await act(async () => subscribe().receive({ resourceId: 'one' }));
  expect(requests).toHaveBeenCalledTimes(1);
});

it('refreshes on change and reconnect notifications and removes its listener', async () => {
  let records = entries();
  const requests = vi.fn(() => HttpResponse.json(records));
  server.use(http.get(endpoint, requests));
  const { rerender } = renderCitadel(<Probe />);
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Healthy'));
  expect(realtime.options?.groupName).toBe('container-health:one');
  const { receive, off, connection } = subscribe();
  records = entries('Unhealthy');
  await act(async () => receive({ resourceId: 'one' }));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Unhealthy'));
  realtime.connected = false;
  rerender(<Probe />);
  expect(screen.getByTestId('history')).toHaveTextContent('Unhealthy');
  realtime.connected = true;
  rerender(<Probe />);
  records = [];
  // Rejoining sends the same invalidation to recover records missed offline.
  await act(async () => receive({ resourceId: 'one' }));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('"entries":[]'));
  expect(requests).toHaveBeenCalledTimes(3);
  realtime.options!.removeEventListeners!(connection);
  expect(off).toHaveBeenCalledWith('ContainerHealthHistoryChanged', receive);
});

it('coalesces notifications during the first GET into one follow-up without losing updates', async () => {
  const first = deferred();
  let reads = 0;
  const requests = vi.fn(async () => {
    const initial = ++reads === 1;
    if (initial) await first.promise;
    return HttpResponse.json(entries(initial ? 'Healthy' : 'Unhealthy'));
  });
  server.use(http.get(endpoint, requests));
  renderCitadel(<Probe />);
  await waitFor(() => expect(requests).toHaveBeenCalledTimes(1));
  const { receive } = subscribe();
  const notifications = Array.from({ length: 10 }, () => receive({ resourceId: 'one' }));
  expect(requests).toHaveBeenCalledTimes(1);
  await act(async () => {
    first.resolve();
    await Promise.all(notifications);
  });
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Unhealthy'));
  expect(requests).toHaveBeenCalledTimes(2);
});

it('isolates resource changes and ignores notifications for another container', async () => {
  const requests = vi.fn((id: unknown) => HttpResponse.json(entries(id === 'one' ? 'Healthy' : 'Starting')));
  server.use(http.get(endpoint, ({ params }) => requests(params.id)));
  const { rerender } = renderCitadel(<Probe />);
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Healthy'));
  const old = subscribe();
  rerender(<Probe id="two" />);
  expect(screen.getByTestId('history')).not.toHaveTextContent('Healthy');
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Starting'));
  const current = subscribe();
  await act(async () => {
    await current.receive({ resourceId: 'one' });
    await old.receive({ resourceId: 'one' });
  });
  expect(requests).toHaveBeenCalledTimes(2);
  expect(screen.getByTestId('history')).toHaveTextContent('Starting');
});

it.each(['disable', 'unmount'] as const)('does not start a queued refresh after %s', async (action) => {
  const pending = deferred();
  const requests = vi.fn(async () => {
    await pending.promise;
    return HttpResponse.json(entries());
  });
  server.use(http.get(endpoint, requests));
  const view = renderCitadel(<Probe />);
  await waitFor(() => expect(requests).toHaveBeenCalledTimes(1));
  const refresh = subscribe().receive({ resourceId: 'one' });
  if (action === 'disable') view.rerender(<Probe enabled={false} />);
  else view.unmount();
  await act(async () => {
    pending.resolve();
    await refresh;
  });
  expect(requests).toHaveBeenCalledTimes(1);
});

it('reports failed HTTP reads, supports retry, and retains data after a failed refresh', async () => {
  let fail = true;
  server.use(http.get(endpoint, () => (fail ? new HttpResponse(null, { status: 503 }) : HttpResponse.json(entries()))));
  const { user } = renderCitadel(<Probe />);
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('"isError":true'));
  expect(screen.getByTestId('history')).toHaveTextContent('"isLoading":false');
  fail = false;
  await user.click(screen.getByRole('button', { name: 'Retry' }));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Healthy'));
  fail = true;
  await act(async () => subscribe().receive({ resourceId: 'one' }));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('"isError":true'));
  expect(screen.getByTestId('history')).toHaveTextContent('Healthy');
});
