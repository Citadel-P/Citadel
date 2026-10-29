import { renderCitadel } from '@/test/render-citadel';
import { createQueryClient } from '@/query-client-wrapper';
import { TaskSheet } from './task-sheet';

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
