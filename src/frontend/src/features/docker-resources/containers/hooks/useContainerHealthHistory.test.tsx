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
  return <pre data-testid="history">{JSON.stringify(history)}</pre>;
}
function subscribe() {
  const on = vi.fn();
  const off = vi.fn();
  const connection = { on, off } as unknown as RealtimeConnection;
  realtime.options!.setupEventListeners(connection);
  expect(on).toHaveBeenCalledWith('ReceiveContainerHealthHistory', expect.any(Function));
  return { receive: on.mock.calls[0][1], off, connection };
}
const snapshot = (resourceId = 'one', status = 'Healthy') => ({
  resourceId,
  entries: [
    { observedAt: 1234, state: 'Running', origin: 'Inspection', health: { status, reason: 'Recorded change' } },
  ],
});

it('skips the realtime history group while disabled and resumes when enabled', () => {
  const { rerender } = renderCitadel(<Probe enabled={false} />);
  expect(realtime.options?.skip).toBe(true);
  rerender(<Probe />);
  expect(realtime.options?.skip).toBe(false);
  rerender(<Probe enabled={false} />);
  expect(realtime.options?.skip).toBe(true);
});

it('receives initial history, updates and reconnect snapshots without HTTP requests', async () => {
  const requests = vi.fn(() => HttpResponse.json([]));
  server.use(http.get('http://localhost/api/v1/containers/:id/health-history', requests));
  const { rerender } = renderCitadel(<Probe />);
  expect(realtime.options?.groupName).toBe('container-health:one');
  const { receive, off, connection } = subscribe();
  await act(async () => receive(snapshot()));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Healthy'));
  await act(async () => receive(snapshot('one', 'Unhealthy')));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Unhealthy'));
  realtime.connected = false;
  rerender(<Probe />);
  expect(screen.getByTestId('history')).toHaveTextContent('"isConnected":false');
  expect(screen.getByTestId('history')).toHaveTextContent('Unhealthy');
  realtime.connected = true;
  rerender(<Probe />);
  await act(async () => receive({ resourceId: 'one', entries: [] }));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('"entries":[]'));
  expect(requests).not.toHaveBeenCalled();
  realtime.options!.removeEventListeners!(connection);
  expect(off).toHaveBeenCalledWith('ReceiveContainerHealthHistory', receive);
});

it('isolates containers and ignores snapshots for other resources', async () => {
  const { rerender } = renderCitadel(<Probe />);
  const first = subscribe();
  await act(async () => first.receive(snapshot()));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Healthy'));
  rerender(<Probe id="two" />);
  expect(screen.getByTestId('history')).not.toHaveTextContent('Healthy');
  const second = subscribe();
  await act(async () => second.receive(snapshot('one')));
  expect(screen.getByTestId('history')).not.toHaveTextContent('Healthy');
  await act(async () => second.receive(snapshot('two', 'Starting')));
  await waitFor(() => expect(screen.getByTestId('history')).toHaveTextContent('Starting'));
});
