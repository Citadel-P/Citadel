import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { useAutomationRuns } from './useAutomationRuns';
import { useAutomationActionGroup } from './useAutomationActionGroup';

const id = '00000000-0000-0000-0000-000000000201';
const run = { id: 'run-1', actionId: id, status: 'Running', cleanupPending: false };
function snapshot(fake: FakeRealtimeConnection, actionId: string, runs: (typeof run)[]) {
  fake.emit('AutomationRunWindow', { actionId, ids: runs.map((item) => item.id) });
  runs.forEach((item) => fake.emit('AutomationRunInfoUpdated', item, 'update'));
}
const key = ['listAutomationActionRuns', { id, query: { limit: 50 } }];
function Runs({ actionId = id }: { actionId?: string }) {
  const query = useAutomationRuns(actionId);
  return <output>{JSON.stringify(query.data?.data.runs ?? [])}</output>;
}
function Action() {
  const query = useAutomationActionGroup(id);
  return <output>{query.data?.data.controlState}</output>;
}
function groups(fake: FakeRealtimeConnection) {
  return { connectionFactory: () => fake, startConnection: (connection: RealtimeConnection) => connection.start() };
}

it('replaces the run window, including cleanup and empty reconnect snapshots, without GETs', async () => {
  const read = vi.fn(() => HttpResponse.json({ runs: [run] }));
  server.use(http.get(`http://localhost/api/v1/automation/actions/${id}/runs`, read));
  const fake = new FakeRealtimeConnection();
  const { queryClient } = renderCitadel(<Runs />, { groups: groups(fake) });
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Running'));
  await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', `automation-runs:${id}`));
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 3100));
  });
  expect(read).toHaveBeenCalledTimes(1);
  act(() => snapshot(fake, 'other', []));
  expect(screen.getByRole('status')).toHaveTextContent('Running');
  act(() => snapshot(fake, id, [{ ...run, status: 'Failed', cleanupPending: true }]));
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('"cleanupPending":true'));
  act(() => snapshot(fake, id, [{ ...run, status: 'Failed', cleanupPending: false }]));
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('"cleanupPending":false'));
  expect(queryClient.getQueryData(key)).toEqual({ data: { runs: [{ ...run, status: 'Failed' }] } });
  act(() => fake.reconnecting());
  await waitFor(() => expect(fake.listenerCount('AutomationRunInfoUpdated')).toBe(0));
  act(() => fake.reconnected());
  await waitFor(() => expect(fake.listenerCount('AutomationRunInfoUpdated')).toBe(1));
  act(() => snapshot(fake, id, []));
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('[]'));
  expect(read).toHaveBeenCalledTimes(1);
});

it('a late initial GET cannot overwrite a newer run snapshot', async () => {
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const read = vi.fn(async () => {
    await pending;
    return HttpResponse.json({ runs: [run] });
  });
  server.use(http.get(`http://localhost/api/v1/automation/actions/${id}/runs`, read));
  const fake = new FakeRealtimeConnection();
  const { queryClient, unmount } = renderCitadel(<Runs />, { groups: groups(fake) });
  await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(fake.listenerCount('AutomationRunInfoUpdated')).toBe(1));
  const completed = { ...run, status: 'Succeeded' };
  act(() => snapshot(fake, id, [completed]));
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Succeeded'));
  await act(async () => {
    finish();
    await pending;
  });
  expect(queryClient.getQueryData(key)).toEqual({ data: { runs: [completed] } });
  unmount();
  expect(fake.listenerCount('AutomationRunInfoUpdated')).toBe(0);
});

it('action details follow committed snapshots and rejoin without HTTP invalidation', async () => {
  const read = vi.fn(() => HttpResponse.json({ id, controlState: 'Processing' }));
  server.use(http.get(`http://localhost/api/v1/automation/actions/${id}`, read));
  const fake = new FakeRealtimeConnection();
  const { queryClient } = renderCitadel(<Action />, { groups: groups(fake) });
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Processing'));
  await waitFor(() => expect(fake.listenerCount('AutomationActionInfoUpdated')).toBe(1));
  act(() => fake.emit('AutomationActionInfoUpdated', { id, controlState: 'Idle' }, 'update'));
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Idle'));
  expect(queryClient.getQueryData(['getAutomationAction', { id }])).toEqual({ data: { id, controlState: 'Idle' } });
  act(() => fake.reconnecting());
  await waitFor(() => expect(fake.listenerCount('AutomationActionInfoUpdated')).toBe(0));
  act(() => fake.reconnected());
  await waitFor(() => expect(fake.listenerCount('AutomationActionInfoUpdated')).toBe(1));
  expect(read).toHaveBeenCalledTimes(1);
});
