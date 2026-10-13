import { waitFor } from '@testing-library/react';
import { BackupPolicyView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { BackupPolicyRunsTab } from './runs';

const { open, empty, run } = vi.hoisted(() => ({
  open: vi.fn(),
  empty: { runs: [] },
  run: {
    id: 'run',
    backupPolicyId: 'policy',
    policyNameSnapshot: 'Nightly',
    trigger: 'Schedule',
    items: [],
  },
}));
vi.mock('@/lib/atoms', async (original) => ({
  ...(await original<typeof import('@/lib/atoms')>()),
  useTaskSheet: () => ({ open }),
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: () => ({}) }));
vi.mock('@/lib/hooks', async (original) => ({
  ...(await original<typeof import('@/lib/hooks')>()),
  useRead: (endpoint: string) => ({ data: { data: endpoint === 'getBackupRun' ? run : empty }, isLoading: false }),
  useMutate: () => ({ mutateAsync: vi.fn() }),
}));
beforeEach(() => open.mockClear());
it('opens the linked run once even when it is outside the first page of history', async () => {
  const resource = { id: 'policy' } as BackupPolicyView;
  const view = renderCitadel(<BackupPolicyRunsTab resource={resource} />, {
    route: '/backup-policies/edit/policy?runId=run#runs',
  });
  await waitFor(() =>
    expect(open).toHaveBeenCalledWith({
      kind: 'backupRunLogs',
      payload: {
        id: 'run',
        name: 'Nightly',
        trigger: 'Schedule',
        items: [],
      },
    }),
  );
  view.rerender(<BackupPolicyRunsTab resource={resource} />);
  expect(open).toHaveBeenCalledTimes(1);
});
it('does not open a run belonging to another policy', () => {
  renderCitadel(<BackupPolicyRunsTab resource={{ id: 'other' } as BackupPolicyView} />, {
    route: '/backup-policies/edit/other?runId=run#runs',
  });
  expect(open).not.toHaveBeenCalled();
});
