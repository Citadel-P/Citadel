import { screen, within } from '@testing-library/react';
import {
  BackupCoverageStatus as Status,
  ContainerStateStatus,
  ResourceControlState,
  type ContainerView,
  type ResourceBackupSummaryView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { ContainersTable } from './table';

vi.hoisted(() => {
  document.queryCommandSupported ??= () => false;
});
vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: undefined }) }));
vi.mock('./container-info/', () => ({ ImageName: () => null }));
vi.mock('@/features/stacks/backup-cell', () => ({
  StackBackupCell: ({ stackId }: { stackId: string }) => <span data-testid="stack-backup">{stackId}</span>,
}));

const container = (id: string, stack: string, stackId: string | null): ContainerView =>
  ({
    id,
    containerId: id,
    name: id,
    stack,
    stackId,
    platformId: 'platform',
    state: ContainerStateStatus.Running,
    controlState: ResourceControlState.Idle,
    ports: {},
    isSystem: false,
    created: 1,
    updated: 1,
    dockerImageId: 'image',
  }) as ContainerView;

it('uses managed stack coverage only for a group with a single stack identity', () => {
  renderCitadel(
    <ContainersTable
      actions={{}}
      isLoading={false}
      items={[
        container('db', 'wordpress', 'wordpress-id'),
        container('web', 'wordpress', 'wordpress-id'),
        container('unmanaged', 'external', null),
        container('conflicting-one', 'ambiguous', 'stack-one'),
        container('conflicting-two', 'ambiguous', 'stack-two'),
      ]}
    />,
  );
  expect(screen.getByRole('columnheader', { name: 'Backup' })).toBeVisible();
  expect(screen.getAllByTestId('stack-backup')).toHaveLength(1);
  const row = screen.getByRole('button', { name: 'wordpress' }).closest('tr')!;
  expect(within(row).getByTestId('stack-backup')).toHaveTextContent('wordpress-id');
  const external = screen.getByRole('button', { name: 'external' }).closest('tr')!;
  expect(within(external).getByRole('button', { name: 'Backups: Coverage unavailable' })).toBeVisible();
});

it('shows deduplicated unmanaged coverage and storage-specific actions in the shared card', async () => {
  const summary: ResourceBackupSummaryView = {
    platformId: 'platform',
    status: Status.Warning,
    unavailable: false,
    hasResourcePolicy: false,
    canConfigure: true,
    canConfigureBindMount: true,
    policies: [],
    warnings: [],
    reasons: [],
    volumes: [
      { name: 'shared', status: Status.Protected, policies: [], includedInResourcePolicy: false },
      { name: 'uploads', status: Status.Unprotected, policies: [], includedInResourcePolicy: false },
    ],
  };
  const items = ['db', 'web'].map((id) => ({ ...container(id, 'external', null), backupCoverage: summary }));
  const view = renderCitadel(<ContainersTable actions={{}} isLoading={false} items={items} />);
  const row = screen.getByRole('button', { name: 'external' }).closest('tr')!;
  await view.user.hover(within(row).getByRole('button', { name: 'Backups: Partially protected' }));
  const card = await screen.findByRole('dialog', { name: 'Backups' });
  expect(within(card).getByText('1 of 2 volumes protected')).toBeVisible();
  expect(within(card).getByText('Storage used by the containers shown in this group.')).toBeVisible();
  const link = new URL(
    within(card).getByRole('link', { name: 'Configure backup for uploads' }).getAttribute('href')!,
    'http://localhost',
  );
  expect(link.searchParams.get('sourceType')).toBe('DockerVolume');
  expect(link.searchParams.get('volumeName')).toBe('uploads');
  expect(link.searchParams.has('stackId')).toBe(false);
  expect(link.searchParams.get('returnTo')).toBe('/platforms/platform/containers');
  view.rerender(
    <ContainersTable
      actions={{}}
      isLoading={false}
      items={items.map((item) => ({ ...item, backupCoverage: { ...summary, status: Status.Failed } }))}
    />,
  );
  expect(within(row).getByRole('button', { name: 'Backups: Backup failed' })).toBeVisible();
});
