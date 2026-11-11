import { renderCitadel } from '@/test/render-citadel';
import { createQueryClient } from '@/query-client-wrapper';
import { TaskSheet } from './task-sheet';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';

const mocks = vi.hoisted(() => ({ kind: 'stack', failed: false }));
vi.mock('@/lib/atoms', () => ({
  useTaskSheet: () => ({
    state: {
      open: true,
      task: { kind: mocks.kind, payload: { id: 'resource-1', stackId: 'resource-1', name: 'Example' } },
    },
    close: vi.fn(),
  }),
}));
vi.mock('@/lib/hooks', async (original) => ({
  ...(await original<typeof import('@/lib/hooks')>()),
  useStreamProgress: () => ({
    status: mocks.failed ? 'error' : 'success',
    isSuccess: !mocks.failed,
    error: mocks.failed ? 'Failed' : undefined,
    logs: [],
    elapsedLabel: '1',
  }),
}));
vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null, MonacoEditor: () => null }));
vi.mock('@/components/custom/log-viewer', () => ({ LogViewer: () => null }));

it.each([
  ['stack', []],
  ['stackRollback', []],
  ['backupRun', ['getPlatformBackupSummaries']],
  ['backupRestoreRun', ['getBackupRun']],
] as const)('completion of %s refreshes only reads without live snapshots', (kind, expected) => {
  mocks.kind = kind;
  mocks.failed = false;
  const queryClient = createQueryClient();
  const invalidate = vi.spyOn(queryClient, 'invalidateQueries');
  renderCitadel(<TaskSheet type="Stack" />, { queryClient });
  expect(invalidate.mock.calls.map(([filters]) => filters?.queryKey?.[0])).toEqual(expected);
});

it.each(['backupRun', 'backupRestoreRun'])('failure of %s keeps uncovered reads fresh', (kind) => {
  mocks.kind = kind;
  mocks.failed = true;
  const queryClient = createQueryClient();
  const invalidate = vi.spyOn(queryClient, 'invalidateQueries');
  renderCitadel(<TaskSheet type="BackupPolicy" />, { queryClient });
  expect(invalidate.mock.calls.map(([filters]) => filters?.queryKey?.[0])).toEqual([
    kind === 'backupRun' ? 'getPlatformBackupSummaries' : 'getBackupRun',
  ]);
});

it('refreshes an open restart activity after reconnect without reading during shutdown', async () => {
  mocks.kind = 'activity';
  const connection = new FakeRealtimeConnection();
  let completed = false;
  const read = vi.fn(() =>
    HttpResponse.json({
      id: 'resource-1',
      resourceType: 'Instance',
      resourceId: 'instance-1',
      resourceName: 'Instance',
      eventType: 'InstanceRestarted',
      status: completed ? 'Success' : 'Information',
      createdAt: '2026-11-11T17:18:00Z',
      actorType: 'User',
      actorName: 'Administrator',
      info: {
        $type: 'InstanceRestarted',
        phase: completed ? 'Completed' : 'Requested',
        completedAt: completed ? '2026-11-11T17:18:11Z' : null,
      },
    }),
  );
  server.use(
    http.get('http://localhost/api/v1/activities/resource-1', read),
    http.get('http://localhost/api/v1/profile/preferences', () =>
      HttpResponse.json({ dateDisplay: 'Exact', timeZone: 'UTC', dateTimeFormat: 'TwentyFourHour' }),
    ),
  );
  renderCitadel(<TaskSheet type="Activity" />, {
    groups: { connectionFactory: () => connection },
  });
  expect(await screen.findByText('Core restart requested. Waiting for startup to complete.')).toBeVisible();
  await waitFor(() => expect(connection.start).toHaveBeenCalledOnce());
  expect(read).toHaveBeenCalledOnce();

  act(() => connection.reconnecting());
  completed = true;
  expect(read).toHaveBeenCalledOnce();
  expect(screen.getByText('Core restart requested. Waiting for startup to complete.')).toBeVisible();

  act(() => connection.reconnected());
  expect(await screen.findByText(/Core restarted successfully/)).toBeVisible();
  expect(screen.queryByText('Core restart requested. Waiting for startup to complete.')).not.toBeInTheDocument();
  expect(read).toHaveBeenCalledTimes(2);
});
