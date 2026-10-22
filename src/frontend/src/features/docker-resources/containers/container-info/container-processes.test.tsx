import { useEffect } from 'react';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import type { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { ContainerProcesses } from './container-processes';

const realtime = vi.hoisted(() => ({
  listeners: new Map<string, (value: unknown) => void | Promise<void>>(),
  groups: new Set<string>(),
  connected: true,
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: (options: Parameters<typeof useRealtimeGroup>[0]) => {
    const { groupName, setupEventListeners, removeEventListeners } = options;
    useEffect(() => {
      const connection = {
        on: (name: string, listener: (value: unknown) => void) => realtime.listeners.set(name, listener),
        off: (name: string) => realtime.listeners.delete(name),
      } as unknown as RealtimeConnection;
      realtime.groups.add(groupName as string);
      setupEventListeners(connection);
      if (realtime.connected) {
        void realtime.listeners.get('ContainerProcessesRefresh')?.({
          resourceId: (groupName as string).split(':')[1],
        });
      }
      return () => {
        realtime.groups.delete(groupName as string);
        removeEventListeners?.(connection);
      };
    }, [groupName, setupEventListeners, removeEventListeners]);
    return { isConnected: realtime.connected };
  },
}));
const endpoint = 'http://localhost/api/v1/containers/:id/processes';
const snapshot = (name = 'server') => ({
  running: true,
  titles: ['USER', 'PID', 'PPID', 'TIME', 'COMMAND'],
  processes: [['root', '42', '1', '00:00:01', name]],
  truncated: false,
});
const update = (data: unknown, resourceId = 'one') =>
  act(async () => {
    await realtime.listeners.get('ContainerProcessesUpdated')?.({ resourceId, data });
  });
beforeEach(() => {
  realtime.connected = true;
  realtime.listeners.clear();
  realtime.groups.clear();
});

it('only fetches and subscribes while expanded; initial GET works without realtime', async () => {
  realtime.connected = false;
  const requests = vi.fn(() => HttpResponse.json(snapshot()));
  server.use(http.get(endpoint, requests));
  const { user } = renderCitadel(<ContainerProcesses resourceId="one" />);
  expect(requests).not.toHaveBeenCalled();
  expect(realtime.groups.size).toBe(0);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(await screen.findByText('server')).toBeVisible();
  expect(requests).toHaveBeenCalledTimes(1);
  expect(realtime.groups.has('container-processes:one')).toBe(true);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(realtime.groups.size).toBe(0);
  expect(realtime.listeners.size).toBe(0);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(screen.getByText('server')).toBeVisible();
  expect(requests).toHaveBeenCalledTimes(1);
});

it('reuses the latest live snapshot when reopening, including the group join notification', async () => {
  const requests = vi.fn(() => HttpResponse.json(snapshot('initial')));
  server.use(http.get(endpoint, requests));
  const { user, queryClient } = renderCitadel(<ContainerProcesses resourceId="one" />);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await screen.findByText('initial');
  await update(snapshot('latest'));
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(realtime.groups.size).toBe(0);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(screen.getByText('latest')).toBeVisible();
  expect(screen.queryByText('initial')).not.toBeInTheDocument();
  expect(queryClient.isFetching()).toBe(0);
  expect(requests).toHaveBeenCalledTimes(1);
});

it('refreshes an expired cached snapshot once when reopening', async () => {
  let value = snapshot('initial');
  const requests = vi.fn(() => HttpResponse.json(value));
  server.use(http.get(endpoint, requests));
  const { user, queryClient } = renderCitadel(<ContainerProcesses resourceId="one" />);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await screen.findByText('initial');
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  const key = ['getContainerProcesses', { id: 'one' }];
  queryClient.setQueryData(key, queryClient.getQueryData(key), { updatedAt: Date.now() - 31_000 });
  value = snapshot('fresh');
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(await screen.findByText('fresh')).toBeVisible();
  expect(requests).toHaveBeenCalledTimes(2);
});

it('does not reuse a failed live snapshot as fresh data on reopen', async () => {
  const requests = vi.fn(() => HttpResponse.json(snapshot()));
  server.use(http.get(endpoint, requests));
  const { user } = renderCitadel(<ContainerProcesses resourceId="one" />);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await screen.findByText('server');
  await update(null);
  expect(screen.getByRole('alert')).toHaveTextContent('Processes unavailable');
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await waitFor(() => expect(requests).toHaveBeenCalledTimes(2));
  expect(await screen.findByText('server')).toBeVisible();
});

it('applies live snapshots without HTTP reads, clears stopped data, and recovers failures', async () => {
  const requests = vi.fn(() => HttpResponse.json(snapshot()));
  server.use(http.get(endpoint, requests));
  const { user } = renderCitadel(<ContainerProcesses resourceId="one" />);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await screen.findByText('server');
  expect(screen.queryByRole('button', { name: 'Refresh' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Retry' })).not.toBeInTheDocument();
  await update(snapshot('ignored'), 'other');
  expect(screen.queryByText('ignored')).not.toBeInTheDocument();
  await update(snapshot('worker'));
  expect(screen.getByText('worker')).toBeVisible();
  expect(requests).toHaveBeenCalledTimes(1);
  await update({ ...snapshot(), running: false, processes: [] });
  expect(screen.getByText('Container is stopped.')).toBeVisible();
  expect(screen.queryByText('worker')).not.toBeInTheDocument();
  await update(null);
  expect(screen.getByRole('alert')).toHaveTextContent('Processes unavailable');
  await user.click(screen.getByRole('button', { name: 'Retry' }));
  expect(await screen.findByText('server')).toBeVisible();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Retry' })).not.toBeInTheDocument();
});

it('keeps a newer live snapshot when the initial GET finishes late', async () => {
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const requests = vi.fn(async () => {
    await pending;
    return HttpResponse.json(snapshot('old'));
  });
  server.use(http.get(endpoint, requests));
  const { user, queryClient } = renderCitadel(<ContainerProcesses resourceId="one" />);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await waitFor(() => expect(requests).toHaveBeenCalledTimes(1));
  await update(snapshot('new'));
  await act(async () => {
    finish();
  });
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(screen.getByText('new')).toBeVisible();
  expect(screen.queryByText('old')).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  expect(screen.getByText('new')).toBeVisible();
  expect(requests).toHaveBeenCalledTimes(1);
});

it('fetches a fresh snapshot on reconnect and isolates container changes', async () => {
  let value = snapshot('initial');
  const requests = vi.fn(() => HttpResponse.json(value));
  server.use(http.get(endpoint, requests));
  const { user, rerender } = renderCitadel(<ContainerProcesses resourceId="one" />);
  await user.click(screen.getByRole('button', { name: 'Processes' }));
  await screen.findByText('initial');
  await update(snapshot('live'));
  value = snapshot('reconnected');
  await act(async () => {
    await realtime.listeners.get('ContainerProcessesRefresh')?.({ resourceId: 'one' });
  });
  expect(await screen.findByText('reconnected')).toBeVisible();
  expect(requests).toHaveBeenCalledTimes(2);
  value = snapshot('second');
  rerender(<ContainerProcesses resourceId="two" />);
  expect(screen.queryByText('reconnected')).not.toBeInTheDocument();
  expect(await screen.findByText('second')).toBeVisible();
  expect(realtime.groups.has('container-processes:one')).toBe(false);
  expect(realtime.groups.has('container-processes:two')).toBe(true);
});
