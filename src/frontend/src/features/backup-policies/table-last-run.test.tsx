import { screen, within } from '@testing-library/react';
import { BackupPolicyView, BackupRunStatus, BackupRunView, ResourceControlState } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { BackupPoliciesTable } from './table';

const policy = (status?: BackupRunStatus): BackupPolicyView => ({
  alertOnFailure: true,
  archivedAt: null,
  backupRepositoryId: 'repository',
  capabilities: null,
  createdAt: '2026-10-25T07:00:00Z',
  createdByActorId: 'actor',
  currentRunId: null,
  description: null,
  firstSuccessfulRunAt: null,
  id: 'policy',
  name: 'Enabled policy',
  normalizedName: 'enabled policy',
  enabled: true,
  controlState: ResourceControlState.Idle,
  source: { $type: 'CitadelSystem' },
  cron: null,
  tags: [],
  keepLastSuccessful: 14,
  lastScheduledRunAt: null,
  rowVersion: 1,
  runAsActorId: 'actor',
  timeZone: 'UTC',
  timeoutSeconds: 300,
  updatedAt: '2026-10-25T08:01:00Z',
  webhook: null,
  latestRun: status
    ? ({ status, queuedAt: '2026-10-25T08:00:00Z', completedAt: '2026-10-25T08:01:00Z' } as BackupRunView)
    : null,
});

it('shows the failed run separately from the enabled policy', async () => {
  const { user } = renderCitadel(
    <BackupPoliciesTable items={[policy(BackupRunStatus.Failed)]} actions={{}} isLoading={false} />,
  );
  expect(screen.getByRole('columnheader', { name: 'Last Run' })).toBeVisible();
  const enabled = screen.getByRole('link', { name: 'Enabled policy' }).previousElementSibling!;
  await user.hover(enabled);
  expect(await screen.findByRole('tooltip')).toHaveTextContent('Enabled');
  await user.unhover(enabled);

  const result = screen.getByRole('img', { name: 'Last run: Failed' });
  expect(result.querySelector('svg')).toHaveClass('text-red-500');
  await user.hover(result);
  expect(await screen.findByRole('tooltip')).toHaveTextContent('Failed');
});

it.each([
  [BackupRunStatus.Succeeded, 'Succeeded', 'text-green-500'],
  [BackupRunStatus.SucceededWithWarnings, 'Succeeded with warnings', 'text-orange-500'],
  [BackupRunStatus.TimedOut, 'Timed out', 'text-red-500'],
  [BackupRunStatus.Interrupted, 'Interrupted', 'text-red-500'],
])('shows the latest %s result after a table update', (status, label, color) => {
  const view = renderCitadel(<BackupPoliciesTable items={[policy()]} actions={{}} isLoading={false} />);
  expect(screen.getByText('No runs')).toBeVisible();
  view.rerender(<BackupPoliciesTable items={[policy(status)]} actions={{}} isLoading={false} />);
  expect(screen.queryByText('No runs')).not.toBeInTheDocument();
  const row = screen.getByRole('link', { name: 'Enabled policy' }).closest('tr')!;
  expect(
    within(row)
      .getByRole('img', { name: `Last run: ${label}` })
      .querySelector('svg'),
  ).toHaveClass(color);
});
