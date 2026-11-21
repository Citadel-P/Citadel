import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { BackupRepositoryStatus, type BackupRepositoryView } from '@/api/generated/api.types';
import { BackupRepositoriesSection } from '@/features/backup-policies/repositories';
import { useRepositoryOperations } from '../operations';
import { useBackupRepositoriesGroup } from './useBackupRepositoriesGroup';
import { useBackupRepositoryGroup } from './useBackupRepositoryGroup';
import { GROUP_CAPACITY_ERROR } from '@/lib/realtime-connection';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';

const repository = {
  id: '00000000-0000-0000-0000-000000000102',
  name: 'Archive',
  capabilities: { canRead: true, canWrite: true, canExecute: true },
  spec: { $type: 'FileSystem', location: 'Core', path: '/backups' },
  status: BackupRepositoryStatus.Unknown,
} as BackupRepositoryView;

function Probe() {
  const list = useBackupRepositoriesGroup();
  const detail = useBackupRepositoryGroup(repository.id);
  const operations = useRepositoryOperations(detail.data?.data);
  return (
    <>
      <output>
        {list.repositories?.[0]?.status} / {detail.data?.data.status}
      </output>
      <button disabled={!operations.canRun('initialize')} onClick={() => void operations.run('initialize')}>
        Initialize
      </button>
      <button disabled={!operations.canRun('check')}>Check</button>
      <button disabled={!operations.canRun('prune')}>Prune</button>
    </>
  );
}

it.each(['live', 'disabled', 'interrupted', 'capacity', 'failed'] as const)(
  'reconciles repository maintenance with %s realtime',
  async (mode) => {
    // After an interruption the authoritative list can equal its first HTTP response,
    // even though the user saw a different status over the socket in between.
    let row = mode === 'interrupted' ? { ...repository, status: BackupRepositoryStatus.Ready } : repository;
    const list = vi.fn(() => HttpResponse.json({ repositories: [row], capabilities: row.capabilities }));
    const detail = vi.fn(() => HttpResponse.json(row));
    const path = '*/api/v1/backupRepositories';
    const connection = new FakeRealtimeConnection();
    const detailGroup = 'backup-repository:' + row.id;
    connection.invoke.mockImplementation(async (method, name) => {
      if (method === 'JoinGroup' && name === detailGroup) {
        if (mode === 'capacity') throw Error(GROUP_CAPACITY_ERROR);
        if (mode === 'failed') throw Error('Join failed');
      }
    });
    let finish!: () => void;
    const initializing = new Promise<void>((resolve) => {
      finish = resolve;
    });
    const initialize = vi.fn(async () => {
      await initializing;
      row = { ...repository, status: BackupRepositoryStatus.Ready };
      return new HttpResponse(null, { status: 204 });
    });
    server.use(
      http.get(path, list),
      http.get(path + '/' + row.id, detail),
      http.post(path + '/' + row.id + '/initialize', initialize),
    );
    const { user } = renderCitadel(<Probe />, {
      groups: {
        realtimeTransport: mode === 'disabled' ? 'None' : 'WebSocketV1',
        connectionFactory: () => connection,
        startConnection: (c) => c.start(),
      },
    });
    await screen.findByText(mode === 'interrupted' ? 'Ready / Ready' : 'Unknown / Unknown');
    if (mode !== 'disabled') {
      await waitFor(() => expect(connection.invoke).toHaveBeenCalledWith('JoinGroup', detailGroup));
      expect(connection.invoke).toHaveBeenCalledWith('JoinGroup', 'backup-repositories');
    }
    if (mode === 'interrupted') {
      row = repository;
      await act(async () => connection.emit('BackupRepositoryInfoUpdated', row, 'update'));
      await screen.findByText('Unknown / Unknown');
    }
    const button = screen.getByRole('button', { name: 'Initialize' });
    await user.click(button);
    await waitFor(() => expect(initialize).toHaveBeenCalledOnce());
    if (mode === 'interrupted') await act(async () => connection.reconnecting());
    await act(async () => finish());

    if (mode === 'live') {
      await waitFor(() => expect(button).toBeEnabled());
      expect(screen.getByText('Unknown / Unknown')).toBeVisible();
    }
    if (mode === 'capacity' || mode === 'failed') await screen.findByText('Unknown / Ready');
    if (mode === 'live' || mode === 'capacity' || mode === 'failed')
      await act(async () => connection.emit('BackupRepositoryInfoUpdated', row, 'update'));

    await screen.findByText('Ready / Ready');
    expect(button).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Check' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Prune' })).toBeEnabled();
    expect(list).toHaveBeenCalledTimes(mode === 'disabled' || mode === 'interrupted' ? 2 : 1);
    expect(detail).toHaveBeenCalledTimes(mode === 'live' ? 1 : 2);
  },
);

it('shows an edited repository in the mounted list after saving with realtime disabled', async () => {
  let row = { ...repository, status: BackupRepositoryStatus.Ready, passwordSecretId: 'password' };
  const list = vi.fn(() => HttpResponse.json({ repositories: [row], capabilities: row.capabilities }));
  server.use(
    http.get('*/api/v1/backupRepositories', list),
    http.get('*/api/v1/resourceBindings/secrets', () => HttpResponse.json({ secrets: [] })),
    http.patch('*/api/v1/backupRepositories/' + row.id, async ({ request }) => {
      const { name } = (await request.json()) as { name: string };
      row = { ...row, name };
      return HttpResponse.json({ ...row, capabilities: null });
    }),
  );
  const { user } = renderCitadel(<BackupRepositoriesSection />, { groups: { realtimeTransport: 'None' } });
  await user.click(await screen.findByText(row.name));
  const dialog = within(screen.getByRole('dialog'));
  const name = dialog.getByRole('textbox', { name: 'Name' });
  await user.clear(name);
  await user.type(name, 'Daily backups');
  await user.click(dialog.getByRole('button', { name: 'Save changes' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(await screen.findByText('Daily backups')).toBeVisible();
  expect(list).toHaveBeenCalledTimes(2);
});
