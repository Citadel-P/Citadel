import { screen, within } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { BackupCoverageStatus, type VolumeView } from '@/api/generated/api.types';
import { VolumesTable } from './table';
import { VolumeComponents } from './index';
import { RegularResourceView } from '@/pages/regular-resource';

const data = vi.hoisted(() => ({ items: [] as VolumeView[] }));
vi.mock('./hooks/useVolumesGroup', () => ({
  useVolumesGroup: () => ({ volumes: { volumes: data.items }, isLoading: false }),
}));

vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: { id: 'platform' } }) }));

vi.mock('./volume-browser-sheet', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./volume-browser-sheet')>()),
  VolumeBrowserSheet: () => null,
}));

const volume = (name: string, status?: BackupCoverageStatus): VolumeView => ({
  id: name,
  isSystem: false,
  capabilities: null,
  clusterVolume: null,
  contentAccessAllowed: true,
  createdAt: '2026-10-14T00:00:00Z',
  dockerNodeId: null,
  nodeHostname: null,
  isStale: false,
  staleReason: null,
  status: {},
  usageData: null,
  name,
  driver: 'local',
  mountpoint: `/data/${name}`,
  scope: 'local',
  options: {},
  labels: {},
  inUse: true,
  containers: [],
  backupCoverage: status ? { status, policyCount: 0, policies: [], canConfigure: true, reasons: [] } : undefined,
});

it('marks daemon-classified system volumes without inferring ownership from names or content access', () => {
  renderCitadel(
    <VolumesTable
      items={[
        { ...volume('core-data'), isSystem: true },
        { ...volume('postgres-data'), isSystem: true, contentAccessAllowed: false },
        volume('citadel-data'),
        { ...volume('ordinary-data'), contentAccessAllowed: false },
      ]}
      actions={{}}
      isLoading={false}
    />,
  );
  for (const name of ['core-data', 'postgres-data']) {
    expect(within(screen.getByText(name).closest('tr')!).getByLabelText('Citadel system volume')).toBeVisible();
  }
  for (const name of ['citadel-data', 'ordinary-data']) {
    expect(
      within(screen.getByText(name).closest('tr')!).queryByLabelText('Citadel system volume'),
    ).not.toBeInTheDocument();
  }
});

it('filters volume backup coverage in the shared toolbar and follows new snapshots', async () => {
  data.items = [
    volume('protected-data', BackupCoverageStatus.Protected),
    volume('unprotected-data', BackupCoverageStatus.Unprotected),
    volume('unknown-data'),
  ];
  const view = () => (
    <RegularResourceView
      type="Volume"
      Components={{ ...VolumeComponents, DropdownActions: {}, GroupActions: undefined }}
      showTaskSheet={false}
    />
  );
  const { user, rerender } = renderCitadel(view(), { route: '/platforms/platform/volumes' });
  expect(screen.getByPlaceholderText('Search volumes…')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Add Volume' })).toBeVisible();
  expect(
    within(screen.getByRole('region', { name: 'Volumes filters' })).getByRole('button', { name: 'Backup Filter' }),
  ).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Backup Filter' }));
  await user.click(screen.getByRole('option', { name: 'Protected' }));
  expect(screen.getByText('protected-data')).toBeVisible();
  expect(screen.queryByText('unprotected-data')).not.toBeInTheDocument();
  expect(screen.queryByText('unknown-data')).not.toBeInTheDocument();
  await user.click(screen.getByRole('option', { name: 'Coverage unavailable' }));
  await user.keyboard('{Escape}');
  expect(screen.getByText('unknown-data')).toBeVisible();

  data.items = data.items.map((item) =>
    item.name === 'protected-data' ? volume(item.name, BackupCoverageStatus.Failed) : item,
  );
  rerender(view());
  expect(screen.queryByText('protected-data')).not.toBeInTheDocument();
  expect(screen.getByText('unknown-data')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Remove Coverage unavailable backup filter' }));
  expect(screen.getByText('No volumes match the current filters.')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Remove Protected backup filter' }));
  expect(screen.getByText('protected-data')).toBeVisible();
  expect(screen.getByText('unprotected-data')).toBeVisible();
  expect(screen.getByText('unknown-data')).toBeVisible();
});

it('shows shared backup status labels and colors, including unavailable coverage', () => {
  renderCitadel(
    <VolumesTable
      items={[
        volume('scheduled-data', BackupCoverageStatus.Protected),
        volume('unscheduled-data', BackupCoverageStatus.Warning),
        volume('failed-data', BackupCoverageStatus.Failed),
        volume('unprotected-data', BackupCoverageStatus.Unprotected),
        volume('unknown-data'),
      ]}
      actions={{}}
      isLoading={false}
    />,
  );
  expect(screen.getByRole('button', { name: 'Backups for scheduled-data: All volumes protected' })).toHaveClass(
    'text-emerald-600',
  );
  expect(screen.getByRole('button', { name: 'Backups for unscheduled-data: Needs attention' })).toHaveClass(
    'text-amber-700',
  );
  expect(screen.getByRole('button', { name: 'Backups for failed-data: Backup failed' })).toHaveClass(
    'text-destructive',
  );
  expect(screen.getByRole('button', { name: 'Backups for unprotected-data: Volumes unprotected' })).toHaveClass(
    'text-amber-700',
  );
  expect(screen.getByRole('button', { name: 'Backups for unknown-data: Coverage unavailable' })).toHaveClass(
    'text-muted-foreground',
  );
});

it('opens the shared card on hover and preserves platform, volume, node, and return location', async () => {
  const item = { ...volume('db & uploads', BackupCoverageStatus.Unprotected), dockerNodeId: 'node-a' };
  const returnTo = '/platforms/platform/volumes?search=db';
  const { user } = renderCitadel(<VolumesTable items={[item]} actions={{}} isLoading={false} />, { route: returnTo });
  await user.hover(screen.getByRole('button', { name: /Backups for db & uploads/ }));
  const card = await screen.findByRole('dialog', { name: 'Backups' });
  const link = within(card).getByRole('link', { name: 'Configure backup for db & uploads' });
  const url = new URL(link.getAttribute('href')!, 'http://localhost');
  expect(url.pathname).toBe('/backup-policies/add');
  expect(Object.fromEntries(url.searchParams)).toEqual({
    sourceType: 'DockerVolume',
    platformId: 'platform',
    volumeName: 'db & uploads',
    dockerNodeId: 'node-a',
    returnTo,
  });
});

it('updates an open card with policy and schedule details from new volume data', async () => {
  const item = volume('data', BackupCoverageStatus.Unprotected);
  const actions = {};
  const { user, rerender } = renderCitadel(<VolumesTable items={[item]} actions={actions} isLoading={false} />, {
    route: '/platforms/platform/volumes',
  });
  await user.click(screen.getByRole('button', { name: /Backups for data/ }));
  const policies = [{ id: 'policy', name: 'Daily data', enabled: true, scheduleConfigured: false, canWrite: true }];
  rerender(
    <VolumesTable
      items={[
        {
          ...item,
          backupCoverage: {
            ...item.backupCoverage!,
            status: BackupCoverageStatus.Warning,
            policyCount: 1,
            policies,
            lastSuccessfulRunAt: '2026-10-14T10:00:00Z',
          },
        },
      ]}
      actions={actions}
      isLoading={false}
    />,
  );
  const card = await screen.findByRole('dialog', { name: 'Backups' });
  expect(within(card).getByRole('link', { name: 'Daily data' })).toHaveAttribute(
    'href',
    '/backup-policies/edit/policy?returnTo=%2Fplatforms%2Fplatform%2Fvolumes',
  );
  expect(within(card).getByRole('link', { name: 'Edit schedule' })).toBeVisible();
  expect(within(card).getByText(/Most recent successful backup:/)).toBeVisible();
  expect(within(card).queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
});

it('does not offer configuration when the actor cannot create policies', async () => {
  const item = volume('read-only', BackupCoverageStatus.Unprotected);
  item.backupCoverage!.canConfigure = false;
  const { user } = renderCitadel(<VolumesTable items={[item]} actions={{}} isLoading={false} />);
  await user.click(screen.getByRole('button', { name: /Backups for read-only/ }));
  expect(
    within(await screen.findByRole('dialog')).queryByRole('link', { name: /Configure backup/ }),
  ).not.toBeInTheDocument();
});
