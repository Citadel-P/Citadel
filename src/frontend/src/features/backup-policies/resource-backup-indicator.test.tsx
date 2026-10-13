import { screen, waitFor } from '@testing-library/react';
import {
  BackupCoverageStatus as Status,
  BackupCoverageReason,
  ResourceBackupSummaryView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { ResourceBackupIndicator } from './resource-backup-indicator';

const source = { $type: 'Stack' as const, stackId: 'stack' };
const coverage: ResourceBackupSummaryView = {
  status: Status.Unprotected,
  platformId: 'platform',
  canConfigure: true,
  unavailable: false,
  hasResourcePolicy: false,
  policies: [],
  warnings: [],
  reasons: [],
  volumes: [{ name: 'data', includedInResourcePolicy: true, status: Status.Unprotected, policies: [] }],
};

it.each([
  [[Status.Protected], 'All volumes protected', Status.Protected],
  [[Status.Protected, Status.Unprotected], 'Partially protected', Status.Warning],
  [[Status.Unprotected], 'Volumes unprotected', Status.Unprotected],
  [[Status.Warning, Status.Protected], 'Needs attention', Status.Warning],
  [[Status.Warning, Status.Failed], 'Backup failed', Status.Failed],
  [[Status.Protected, null], 'Coverage unavailable', null],
  [[Status.NotApplicable], 'No eligible volumes', Status.NotApplicable],
  [[], 'No eligible volumes', Status.NotApplicable],
] as const)('summarizes %j as %s', (statuses, label, aggregate) => {
  renderCitadel(
    <ResourceBackupIndicator
      source={source}
      coverage={{
        ...coverage,
        status: aggregate,
        volumes: statuses.map((status, i) => ({ ...coverage.volumes[0], name: `v${i}`, status })),
      }}
    />,
  );
  expect(screen.getByRole('button', { name: `Backups: ${label}` })).toHaveAttribute('aria-expanded', 'false');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

it('opens on hover without stealing focus and allows moving into the card', async () => {
  const { user } = renderCitadel(<ResourceBackupIndicator source={source} coverage={coverage} />);
  const trigger = screen.getByRole('button', { name: 'Backups: Volumes unprotected' });
  await user.hover(trigger);
  const card = await screen.findByRole('dialog', { name: 'Backups' });
  expect(trigger).not.toHaveFocus();
  await user.hover(card);
  expect(screen.getByRole('link', { name: 'Configure backup' })).toBeVisible();
  await user.unhover(card);
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
});

it('opens on focus, lets the keyboard enter the card, and restores focus on Escape', async () => {
  const { user } = renderCitadel(<ResourceBackupIndicator source={source} coverage={coverage} />);
  const trigger = screen.getByRole('button', { name: 'Backups: Volumes unprotected' });
  await user.tab();
  expect(trigger).toHaveFocus();
  expect(screen.getByRole('dialog', { name: 'Backups' })).toBeVisible();
  await user.tab();
  expect(screen.getByRole('dialog')).toHaveFocus();
  await user.tab();
  expect(screen.getByRole('link', { name: 'Configure backup' })).toHaveFocus();
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(trigger).toHaveFocus();
  await user.keyboard('{Enter}');
  expect(screen.getByRole('dialog')).toBeVisible();
});

it('pins on click, toggles closed, and dismisses on an outside click', async () => {
  const { user } = renderCitadel(
    <>
      <ResourceBackupIndicator source={source} coverage={coverage} />
      <button>Outside</button>
    </>,
  );
  const trigger = screen.getByRole('button', { name: 'Backups: Volumes unprotected' });
  await user.click(trigger);
  expect(screen.getByRole('dialog')).toBeVisible();
  await user.click(trigger);
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  await user.click(trigger);
  await user.click(screen.getByRole('button', { name: 'Outside' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

it('updates an open card and its shield from new resource coverage', async () => {
  const { user, rerender } = renderCitadel(<ResourceBackupIndicator source={source} coverage={coverage} />);
  await user.click(screen.getByRole('button', { name: /Backups:/ }));
  expect(screen.getByRole('status')).toHaveTextContent('0 of 1 volumes protected');
  rerender(
    <ResourceBackupIndicator
      source={source}
      coverage={{
        ...coverage,
        status: Status.Protected,
        volumes: [{ ...coverage.volumes[0], status: Status.Protected }],
      }}
    />,
  );
  expect(screen.getByRole('status')).toHaveTextContent('1 of 1 volumes protected');
  expect(screen.getByRole('button', { name: 'Backups: All volumes protected' })).toHaveAttribute(
    'aria-expanded',
    'true',
  );
});

it('does not report stale protected volumes as protected when coverage is unavailable', async () => {
  const { user } = renderCitadel(
    <ResourceBackupIndicator
      source={source}
      coverage={{ ...coverage, unavailable: true, volumes: [{ ...coverage.volumes[0], status: Status.Protected }] }}
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Backups: Coverage unavailable' }));
  expect(screen.getByText('Backup coverage is unavailable.')).toBeVisible();
  expect(screen.queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
});

it.each([
  { $type: 'Deployment' as const, deploymentId: 'deployment' },
  { $type: 'Stack' as const, stackId: 'stack' },
  { $type: 'SwarmService' as const, swarmServiceId: 'service' },
  { $type: 'Container' as const, platformId: 'platform', containerId: 'container' },
])('shows manual-only coverage in amber for $type and turns green after a scheduled success', (resource) => {
  const policy = { id: 'policy', name: 'Manual', enabled: true, scheduleConfigured: false, canWrite: true };
  const snapshot = {
    ...coverage,
    status: Status.Warning,
    policies: [policy],
    volumes: [
      {
        ...coverage.volumes[0],
        status: Status.Warning,
        policies: [policy],
        lastSuccessfulRunAt: '2026-10-12T10:00:00Z',
      },
    ],
  };
  const view = renderCitadel(<ResourceBackupIndicator source={resource} coverage={snapshot} />);
  expect(screen.getByRole('button', { name: 'Backups: Automatic backups not configured' })).toHaveClass(
    'text-amber-700',
  );
  view.rerender(
    <ResourceBackupIndicator
      source={resource}
      coverage={{
        ...snapshot,
        status: Status.Protected,
        policies: [{ ...policy, scheduleConfigured: true }],
        volumes: [{ ...snapshot.volumes[0], status: Status.Protected }],
      }}
    />,
  );
  expect(screen.getByRole('button', { name: 'Backups: All volumes protected' })).toHaveClass('text-emerald-600');
});

it('uses the server aggregate even when successful volumes are present', () => {
  renderCitadel(
    <ResourceBackupIndicator
      source={source}
      coverage={{
        ...coverage,
        status: Status.Warning,
        reasons: [BackupCoverageReason.SchedulingUnavailable],
        volumes: [{ ...coverage.volumes[0], status: Status.Protected }],
      }}
    />,
  );
  expect(screen.getByRole('button', { name: 'Backups: Automatic backups paused' })).toHaveClass('text-amber-700');
});
