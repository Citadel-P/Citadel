import { screen, within } from '@testing-library/react';
import { BackupCoverageStatus as Status, ResourceBackupSummaryView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { BackupStatusFilter, useBackupStatusFilter } from './table-coverage';
import { ResourceHeader } from '@/pages/resource-header';
import { DeploymentsTable } from '@/features/deployments/table';
import { useLocation, useNavigate } from 'react-router';

const coverage = (status: Status): ResourceBackupSummaryView => ({
  status,
  platformId: 'platform',
  unavailable: false,
  warnings: [],
  reasons: [],
  policies: [],
  volumes: [],
  canConfigure: false,
  hasResourcePolicy: false,
});
const items = [
  { name: 'Scheduled', backupCoverage: coverage(Status.Protected) },
  { name: 'Manual', backupCoverage: coverage(Status.Warning) },
  { name: 'Broken', backupCoverage: coverage(Status.Failed) },
];
function Table({ rows = items }: { rows?: typeof items }) {
  const filtered = useBackupStatusFilter(rows);
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <>
      <ResourceHeader type="Deployment" showSearch Extra={BackupStatusFilter} onSearch={() => {}} onAdd={() => {}} />
      <button onClick={() => navigate(-1)}>Previous filters</button>
      <output aria-label="Current query">{location.search}</output>
      {filtered.map((item) => (
        <span key={item.name}>{item.name}</span>
      ))}
    </>
  );
}
beforeEach(() => localStorage.clear());
it('always displays the backup column even with an old hidden-column preference', () => {
  localStorage.setItem('deployments:backup-column', 'false');
  renderCitadel(<DeploymentsTable items={[]} actions={{}} isLoading={false} />);
  expect(screen.getByRole('columnheader', { name: 'Backup' })).toBeVisible();
  expect(screen.queryByRole('checkbox', { name: 'Show backup column' })).not.toBeInTheDocument();
});
it('places the dropdown in the resource header and updates when realtime coverage changes', async () => {
  const view = renderCitadel(<Table />);
  const header = screen.getByRole('region', { name: 'Deployments filters' });
  await view.user.click(within(header).getByRole('button', { name: 'Backup Filter' }));
  await view.user.click(screen.getByRole('option', { name: 'Needs attention / partial coverage' }));
  expect(screen.getByText('Manual')).toBeVisible();
  expect(screen.queryByText('Scheduled')).not.toBeInTheDocument();
  expect(screen.getByLabelText('Current query')).toHaveTextContent('backupStatus=Warning');
  view.rerender(<Table rows={items.map((item) => ({ ...item, backupCoverage: coverage(Status.Protected) }))} />);
  expect(screen.queryByText('Manual')).not.toBeInTheDocument();
});
it('restores the URL filter and clears it without removing other filters', async () => {
  const view = renderCitadel(<Table />, { route: '/deployments?platformId=platform&backupStatus=Warning&tags=demo' });
  expect(screen.queryByText('Scheduled')).not.toBeInTheDocument();
  await view.user.click(
    screen.getByRole('button', { name: 'Remove Needs attention / partial coverage backup filter' }),
  );
  expect(screen.getByText('Scheduled')).toBeVisible();
  expect(screen.getByLabelText('Current query')).toHaveTextContent('?platformId=platform&tags=demo');
});
it('allows returning to all statuses from the dropdown', async () => {
  const view = renderCitadel(<Table />, { route: '/deployments?backupStatus=Warning' });
  await view.user.click(screen.getByRole('button', { name: 'Backup Filter' }));
  await view.user.click(screen.getByRole('button', { name: 'Clear backup filters' }));
  expect(screen.getByText('Scheduled')).toBeVisible();
  expect(screen.getByText('Manual')).toBeVisible();
});
it('ignores unknown backup status values', () => {
  renderCitadel(<Table />, { route: '/deployments?backupStatus=invalid' });
  expect(screen.getByText('Scheduled')).toBeVisible();
  expect(screen.getByText('Manual')).toBeVisible();
});

it('adds statuses with OR matching and keeps the dropdown open for multiple selections', async () => {
  const view = renderCitadel(<Table />, { route: '/deployments?platformId=platform&tags=demo&overview=attention' });
  await view.user.click(screen.getByRole('button', { name: 'Backup Filter' }));
  await view.user.click(screen.getByRole('option', { name: 'Needs attention / partial coverage' }));
  expect(screen.queryByText('Broken')).not.toBeInTheDocument();
  await view.user.click(screen.getByRole('option', { name: 'Backup failed' }));
  expect(screen.getByText('Manual')).toBeVisible();
  expect(screen.getByText('Broken')).toBeVisible();
  expect(screen.queryByText('Scheduled')).not.toBeInTheDocument();
  expect(screen.getByLabelText('Current query')).toHaveTextContent(
    '?platformId=platform&tags=demo&overview=attention&backupStatus=Warning&backupStatus=Failed',
  );
  expect(screen.queryByRole('option', { name: 'Needs attention / partial coverage' })).not.toBeInTheDocument();
  await view.user.click(
    screen.getByRole('button', { name: 'Remove Needs attention / partial coverage backup filter' }),
  );
  expect(screen.queryByText('Manual')).not.toBeInTheDocument();
  await view.user.click(screen.getByRole('button', { name: 'Backup Filter' }));
  expect(screen.getByText('Broken')).toBeVisible();
  await view.user.click(screen.getByRole('button', { name: 'Clear backup filters' }));
  expect(screen.getByText('Scheduled')).toBeVisible();
  expect(screen.getByLabelText('Current query')).toHaveTextContent('?platformId=platform&tags=demo&overview=attention');
});
it('restores multiple statuses, deduplicates them, and removes chips independently', async () => {
  const view = renderCitadel(<Table />, {
    route: '/deployments?backupStatus=Warning&backupStatus=Failed&backupStatus=Warning&backupStatus=invalid',
  });
  expect(screen.getByText('Manual')).toBeVisible();
  expect(screen.getByText('Broken')).toBeVisible();
  expect(screen.queryByText('Scheduled')).not.toBeInTheDocument();
  expect(
    screen.getAllByRole('button', { name: 'Remove Needs attention / partial coverage backup filter' }),
  ).toHaveLength(1);
  await view.user.click(
    screen.getByRole('button', { name: 'Remove Needs attention / partial coverage backup filter' }),
  );
  expect(screen.queryByText('Manual')).not.toBeInTheDocument();
  expect(screen.getByText('Broken')).toBeVisible();
  expect(screen.getByLabelText('Current query')).toHaveTextContent('?backupStatus=Failed');
  await view.user.click(screen.getByRole('button', { name: 'Previous filters' }));
  expect(screen.getByText('Manual')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Remove Needs attention / partial coverage backup filter' })).toBeVisible();
});
