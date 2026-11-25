import { screen, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { BackupCoverageStatus, HealthStatus, type DashboardWorkloadView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { WorkloadTable } from './resource-rows';

it('loads authorized row details on demand and shares the read between health and backup', async () => {
  const row: DashboardWorkloadView = {
    id: 'container-a',
    platformId: 'platform-a',
    name: 'Worker',
    status: 'Running',
    processing: false,
    observedProbes: [HealthStatus.Healthy],
    moreProbes: false,
    backupStatus: BackupCoverageStatus.Protected,
    canViewLogs: false,
  };
  const read = vi.fn(() =>
    HttpResponse.json({
      ...row,
      containerId: 'runtime-a',
      state: 'Running',
      health: { status: 'Healthy', observedAt: Date.now(), reason: 'Readiness probe passed' },
      backupCoverage: null,
    }),
  );
  server.use(
    http.get('*/api/v1/containers/container-a', read),
    http.get('*/api/v1/profile/preferences', () => HttpResponse.json({})),
  );
  const { user } = renderCitadel(<WorkloadTable kind="Container" rows={[row]} />);
  expect(
    within(screen.getByRole('table'))
      .getAllByRole('columnheader')
      .map((h) => h.textContent),
  ).toEqual(['Name', 'Health', 'Backup']);
  expect(read).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Observed probes: Healthy' }));
  expect(await screen.findByText('Readiness probe passed')).toBeVisible();
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Backups: Protected' }));
  expect(await screen.findByText('Backup coverage is unavailable.')).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
});
