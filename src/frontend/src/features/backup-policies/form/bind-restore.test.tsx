import { screen } from '@testing-library/react';
import { BackupRunView, PlatformType } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { BackupRestoreDialog } from './runs';

const { open } = vi.hoisted(() => ({ open: vi.fn() }));
vi.mock('@/lib/atoms', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/atoms')>()),
  useTaskSheet: () => ({ open }),
}));
vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: (endpoint: string, args?: { id?: string }) => ({
    data:
      endpoint === 'getPlatfom'
        ? { data: { id: args?.id, type: args?.id === 'docker' ? PlatformType.Docker : PlatformType.DockerSwarm } }
        : endpoint === 'lookup'
          ? { data: [{ id: 'platform', name: 'Swarm' }] }
          : {
              data: {
                items: [
                  { id: 'node-a', hostname: 'worker-a', status: 'ready', isStale: false },
                  { id: 'node-b', hostname: 'worker-b', status: 'ready', isStale: false },
                  { id: 'node-down', hostname: 'offline-worker', status: 'down', isStale: false },
                  { id: 'node-stale', hostname: 'stale-worker', status: 'ready', isStale: true },
                ],
              },
            },
  }),
}));
const run = {
  id: 'run',
  sourceSnapshot: { $type: 'BindMount', platformId: 'platform', hostPath: '/srv/source', dockerNodeId: 'node-a' },
  items: [
    {
      id: 'item',
      platformId: 'platform',
      hostPath: '/srv/source',
      volumeName: '',
      dockerNodeId: 'node-a',
      status: 'Succeeded',
      resticSnapshotId: 'snapshot',
    },
  ],
} as BackupRunView;

it('requires path confirmation for overwrite and submits a node-specific host target', async () => {
  const { user } = renderCitadel(<BackupRestoreDialog run={run} onClose={() => {}} />);
  expect(screen.getByRole('heading', { name: 'Restore bind mount' })).toBeVisible();
  const input = screen.getByLabelText('Target host path');
  await user.clear(input);
  await user.type(input, '/srv/recovered');
  await user.click(screen.getByRole('switch', { name: 'Allow overwriting existing host files' }));
  expect(screen.getByRole('button', { name: 'Restore' })).toBeDisabled();
  await user.type(screen.getByLabelText('Type the target host path to confirm'), '/srv/recovered');
  await user.click(screen.getByRole('button', { name: 'Restore' }));
  expect(open).toHaveBeenCalledWith({
    kind: 'backupRestoreRun',
    payload: expect.objectContaining({
      targetHostPath: '/srv/recovered',
      targetVolumeName: '',
      targetPlatformId: 'platform',
      targetDockerNodeId: 'node-a',
      sourceBackupRunItemId: 'item',
      overwriteExisting: true,
    }),
  });
});

it('searches ready restore nodes and excludes stale or offline nodes', async () => {
  const { user } = renderCitadel(<BackupRestoreDialog run={run} onClose={() => {}} />);
  await user.click(screen.getByRole('combobox', { name: 'Target Node' }));
  expect(screen.queryByRole('option', { name: 'offline-worker' })).not.toBeInTheDocument();
  expect(screen.queryByRole('option', { name: 'stale-worker' })).not.toBeInTheDocument();
  await user.type(screen.getByPlaceholderText('Search Nodes...'), 'worker-b');
  await user.click(screen.getByRole('option', { name: 'worker-b' }));
  await user.click(screen.getByRole('button', { name: 'Restore' }));
  expect(open).toHaveBeenLastCalledWith({
    kind: 'backupRestoreRun',
    payload: expect.objectContaining({ targetDockerNodeId: 'node-b' }),
  });
});

it('restores a bind snapshot from a mixed workload policy to a host path', async () => {
  const mixedRun = {
    ...run,
    sourceSnapshot: { $type: 'Stack', stackId: 'stack' },
    items: [run.items[0], { ...run.items[0], id: 'volume-item', hostPath: null, volumeName: 'data' }],
  } as BackupRunView;
  const { user } = renderCitadel(<BackupRestoreDialog run={mixedRun} onClose={() => {}} />);
  await user.click(screen.getByRole('combobox', { name: 'Snapshot source' }));
  await user.click(screen.getByRole('option', { name: /\/srv\/source/ }));
  expect(screen.getByRole('heading', { name: 'Restore bind mount' })).toBeVisible();
  const target = screen.getByLabelText('Target host path');
  await user.clear(target);
  await user.type(target, '/srv/recovered');
  await user.click(screen.getByRole('button', { name: 'Restore' }));
  expect(open).toHaveBeenLastCalledWith({
    kind: 'backupRestoreRun',
    payload: expect.objectContaining({
      sourceBackupRunItemId: 'item',
      targetHostPath: '/srv/recovered',
      targetVolumeName: '',
      targetDockerNodeId: 'node-a',
    }),
  });
});

it('selects a successful snapshot and resets overwrite confirmation when its source changes', async () => {
  const volumeRun = {
    ...run,
    sourceSnapshot: { $type: 'Stack', stackId: 'stack' },
    items: [
      { ...run.items[0], id: 'item-a', platformId: 'docker', hostPath: null, volumeName: 'data-a' },
      { ...run.items[0], id: 'item-b', hostPath: null, volumeName: 'data-b', dockerNodeId: 'node-b' },
      { ...run.items[0], id: 'failed', status: 'Failed', volumeName: 'failed-volume' },
    ],
  } as BackupRunView;
  const { user } = renderCitadel(<BackupRestoreDialog run={volumeRun} onClose={() => {}} />);
  await user.click(screen.getByRole('combobox', { name: 'Snapshot Volume' }));
  expect(screen.queryByRole('option', { name: /failed-volume/ })).not.toBeInTheDocument();
  await user.click(screen.getByRole('option', { name: /data-a/ }));
  await user.click(screen.getByRole('switch'));
  expect(screen.getByRole('button', { name: 'Restore' })).toBeDisabled();
  await user.click(screen.getByRole('combobox', { name: 'Snapshot Volume' }));
  await user.type(screen.getByPlaceholderText('Search snapshots...'), 'data-b');
  await user.click(screen.getByRole('option', { name: /data-b/ }));
  expect(screen.queryByRole('switch')).not.toBeInTheDocument();
  expect(screen.queryByLabelText('Type the target volume name to confirm')).not.toBeInTheDocument();
  expect(screen.getByLabelText('Target Volume')).toHaveValue('data-b');
  await user.click(screen.getByRole('button', { name: 'Restore' }));
  expect(open).toHaveBeenLastCalledWith({
    kind: 'backupRestoreRun',
    payload: expect.objectContaining({
      sourceBackupRunItemId: 'item-b',
      targetVolumeName: 'data-b',
      targetDockerNodeId: 'node-b',
      overwriteExisting: false,
    }),
  });
});
