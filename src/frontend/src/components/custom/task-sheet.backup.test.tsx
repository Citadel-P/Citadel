import { screen } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { TaskSheet } from './task-sheet';

const mocks = vi.hoisted(() => ({
  kind: 'backupRunLogs',
  run: {
    status: 'Failed',
    errorMessage: 'No supported volumes or bind mounts were resolved for this resource.',
    errorCode: 'SourceUnavailable',
    items: [],
  },
  logs: '',
  loading: false,
  unavailable: false,
}));
vi.mock('@/lib/atoms', () => ({
  useTaskSheet: () => ({
    state: {
      open: true,
      task: {
        kind: mocks.kind,
        payload: { id: 'run', name: 'wordpress-backup', trigger: 'Manual', status: 'Failed', items: [] },
      },
    },
    close: vi.fn(),
  }),
}));
vi.mock('@/lib/hooks', () => ({
  useRead: (endpoint: string) => ({
    data: { data: endpoint.endsWith('Logs') ? { logs: mocks.logs } : mocks.run },
    isLoading: mocks.loading,
    isError: mocks.unavailable,
  }),
}));
vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null, MonacoEditor: () => null }));
vi.mock('@/components/custom/log-viewer', async (original) => ({
  ...(await original<typeof import('@/components/custom/log-viewer')>()),
  LogViewer: ({ logs }: { logs: string }) => <pre data-testid="execution-log">{logs}</pre>,
}));

beforeEach(() => {
  mocks.kind = 'backupRunLogs';
  mocks.run = {
    status: 'Failed',
    errorMessage: 'No supported volumes or bind mounts were resolved for this resource.',
    errorCode: 'SourceUnavailable',
    items: [],
  };
  mocks.logs = '';
  mocks.loading = false;
  mocks.unavailable = false;
});
it.each(['backupRunLogs', 'backupRestoreRunLogs'])(
  'shows saved pre-execution failure for %s with no log rows',
  (kind) => {
    mocks.kind = kind;
    renderCitadel(<TaskSheet type="BackupPolicy" />);
    expect(screen.getByText(mocks.run.errorMessage)).toBeVisible();
    expect(screen.getByText('No execution logs were recorded for this run.')).toBeVisible();
    expect(screen.queryByTestId('execution-log')).not.toBeInTheDocument();
  },
);
it('keeps engine output visible alongside the saved failure', () => {
  mocks.logs = 'repository connection failed';
  renderCitadel(<TaskSheet type="BackupPolicy" />);
  expect(screen.getByText(mocks.run.errorMessage)).toBeVisible();
  expect(screen.getByTestId('execution-log')).toHaveTextContent(mocks.logs);
});
it('does not present a successful empty log as a failure', () => {
  mocks.run = { status: 'Succeeded', errorMessage: '', errorCode: '', items: [] };
  renderCitadel(<TaskSheet type="BackupPolicy" />);
  expect(screen.queryByText('Run failed')).not.toBeInTheDocument();
  expect(screen.getByText('No execution logs were recorded for this run.')).toBeVisible();
});
it('uses the error code when a historical run has no error message', () => {
  mocks.run.errorMessage = '';
  renderCitadel(<TaskSheet type="BackupPolicy" />);
  expect(screen.getByText('SourceUnavailable')).toBeVisible();
});
it('distinguishes cancelled restores from failures', () => {
  mocks.kind = 'backupRestoreRunLogs';
  mocks.run = { status: 'Cancelled', errorMessage: 'Restore cancelled.', errorCode: 'Cancelled', items: [] };
  renderCitadel(<TaskSheet type="BackupPolicy" />);
  expect(screen.getByText('Run cancelled')).toBeVisible();
  expect(screen.queryByText('Run failed')).not.toBeInTheDocument();
});
it('does not show an empty log result while loading', () => {
  mocks.loading = true;
  renderCitadel(<TaskSheet type="BackupPolicy" />);
  expect(screen.getByText('Loading run details and logs...')).toBeVisible();
  expect(screen.queryByText('No execution logs were recorded for this run.')).not.toBeInTheDocument();
});
it('reports read failures instead of claiming there are no logs', () => {
  mocks.unavailable = true;
  renderCitadel(<TaskSheet type="BackupPolicy" />);
  expect(screen.getByText(/Unable to load complete run details and logs/)).toBeVisible();
  expect(screen.queryByText('No execution logs were recorded for this run.')).not.toBeInTheDocument();
});

it('does not repeat a pre-execution error recorded as the only log line', () => {
  mocks.logs = mocks.run.errorMessage + '\n';
  renderCitadel(<TaskSheet type="BackupPolicy" />);
  expect(screen.getAllByText(mocks.run.errorMessage)).toHaveLength(1);
  expect(screen.queryByTestId('execution-log')).not.toBeInTheDocument();
});

it.each(['Failed', 'TimedOut', 'Interrupted', 'Rejected'])(
  'presents %s as a failure, including when diagnostics are missing',
  (status) => {
    mocks.run = { status, errorMessage: '', errorCode: '', items: [] };
    renderCitadel(<TaskSheet type="BackupPolicy" />);
    expect(screen.getByText('Run failed')).toBeVisible();
    expect(screen.getByText('The run failed without a recorded error message.')).toBeVisible();
    expect(screen.queryByText('Run warning')).not.toBeInTheDocument();
  },
);
