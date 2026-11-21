import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { BackupRepositoryStatus, type BackupRepositoryView } from '@/api/generated/api.types';
import { useRepositoryOperations } from '../operations';
import { useBackupRepositoriesGroup } from './useBackupRepositoriesGroup';
import { useBackupRepositoryGroup } from './useBackupRepositoryGroup';
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
    </>
  );
}

it.each([false, true])(
  'updates the list and detail after maintenance (interrupted connection: %s)',
  async (interrupted) => {
    let row = repository;
    const list = vi.fn(() => HttpResponse.json({ repositories: [row], capabilities: row.capabilities }));
    const detail = vi.fn(() => HttpResponse.json(row));
    const path = '*/api/v1/backupRepositories';
    const connection = new FakeRealtimeConnection();
    const initialize = vi.fn(() => {
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
        connectionFactory: () => connection,
        startConnection: (c) => c.start(),
      },
    });
    await screen.findByText('Unknown / Unknown');
    await waitFor(() => expect(connection.invoke).toHaveBeenCalledWith('JoinGroup', 'backup-repository:' + row.id));
    expect(connection.invoke).toHaveBeenCalledWith('JoinGroup', 'backup-repositories');
    if (interrupted) await act(async () => connection.reconnecting());
    await user.click(screen.getByRole('button', { name: 'Initialize' }));
    await waitFor(() => expect(initialize).toHaveBeenCalledOnce());
    await waitFor(() => expect(screen.getByRole('button', { name: 'Initialize' })).toBeEnabled());
    expect(list).toHaveBeenCalledOnce();
    expect(detail).toHaveBeenCalledOnce();
    expect(screen.getByText('Unknown / Unknown')).toBeVisible();

    if (interrupted) {
      await act(async () => connection.reconnected());
    } else {
      await act(async () => connection.emit('BackupRepositoryInfoUpdated', row, 'update'));
    }
    await screen.findByText('Ready / Ready');
    expect(list).toHaveBeenCalledTimes(interrupted ? 2 : 1);
    expect(detail).toHaveBeenCalledTimes(interrupted ? 2 : 1);
  },
);
