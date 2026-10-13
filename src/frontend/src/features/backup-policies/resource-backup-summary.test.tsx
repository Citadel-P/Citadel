import { screen } from '@testing-library/react';
import {
  BackupCoverageStatus as Status,
  BackupCoverageReason,
  ResourceBackupSummaryView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { ResourceBackupSummary } from './resource-backup-summary';
import { CoverageIndicator } from './coverage-indicator';

const policy = { id: 'policy', name: 'Database nightly' };
const coverage: ResourceBackupSummaryView = {
  status: Status.Unprotected,
  platformId: 'p',
  canConfigure: true,
  unavailable: false,
  hasResourcePolicy: false,
  policies: [policy],
  warnings: [],
  reasons: [],
  volumes: [
    {
      name: 'db',
      includedInResourcePolicy: true,
      status: Status.Protected,
      policies: [policy],
      lastSuccessfulRunAt: '2026-10-12T10:00:00Z',
    },
    { name: 'uploads', includedInResourcePolicy: true, status: Status.Unprotected, policies: [] },
    { name: 'cache', includedInResourcePolicy: true, status: null, policies: [] },
  ],
};
const source = { $type: 'Stack' as const, stackId: 'stack' };

it('renders coverage delivered by the resource model and updates when a new snapshot arrives', () => {
  const { rerender } = renderCitadel(<ResourceBackupSummary source={source} coverage={coverage} />, {
    route: '/stacks/edit/stack#config',
  });
  expect(screen.getByRole('status')).toHaveTextContent('1 of 3 volumes protected');
  expect(screen.getByRole('link', { name: 'Database nightly' })).toHaveAttribute(
    'href',
    '/backup-policies/edit/policy?returnTo=%2Fstacks%2Fedit%2Fstack%23config',
  );
  expect(screen.getByRole('link', { name: 'Configure backup for uploads' }).getAttribute('href')).toContain(
    'volumeName=uploads',
  );
  expect(screen.queryByRole('link', { name: 'Configure backup for cache' })).not.toBeInTheDocument();
  rerender(
    <ResourceBackupSummary
      source={source}
      coverage={{ ...coverage, volumes: coverage.volumes.map((v) => ({ ...v, status: Status.Protected })) }}
    />,
  );
  expect(screen.getByRole('status')).toHaveTextContent('3 of 3 volumes protected');
});

it('offers anonymous Docker volumes their own policy despite an existing deployment policy', () => {
  renderCitadel(
    <ResourceBackupSummary
      source={{ $type: 'Deployment', deploymentId: 'deployment' }}
      coverage={{
        ...coverage,
        hasResourcePolicy: true,
        volumes: [{ ...coverage.volumes[1], includedInResourcePolicy: false }],
      }}
    />,
  );
  expect(screen.getByText('Not included in deployment backups')).toBeVisible();
  expect(screen.getByRole('link', { name: 'Configure backup for uploads' }).getAttribute('href')).toContain(
    'sourceType=DockerVolume',
  );
  expect(screen.queryByRole('link', { name: 'Configure backup' })).not.toBeInTheDocument();
});

it('uses server permissions for configuration links', () => {
  renderCitadel(<ResourceBackupSummary source={source} coverage={{ ...coverage, canConfigure: false }} />);
  expect(screen.queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
});

it('shows an undeployed stack warning without a request error or configuration link', () => {
  renderCitadel(
    <ResourceBackupSummary
      source={source}
      coverage={{
        ...coverage,
        volumes: [],
        policies: [],
        warnings: [
          'No applied Swarm Stack was found. Apply the Stack and synchronize its inventory before configuring backups.',
        ],
      }}
    />,
  );
  expect(screen.getByText(/No applied Swarm Stack was found/)).toBeVisible();
  expect(screen.queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
});

it('distinguishes unavailable coverage from an empty workload', () => {
  renderCitadel(<ResourceBackupSummary source={source} coverage={{ ...coverage, unavailable: true }} />);
  expect(screen.getByText('Backup coverage is unavailable.')).toBeVisible();
  expect(screen.queryByText('No supported storage found')).not.toBeInTheDocument();
});

it('renders empty storage once and preserves distinct warnings', () => {
  const warning = 'No supported volumes or bind mounts were resolved for this resource.';
  const offlineWarning = 'The source Platform is offline or unavailable.';
  renderCitadel(
    <ResourceBackupSummary
      source={source}
      coverage={{ ...coverage, volumes: [], policies: [], warnings: [offlineWarning, offlineWarning] }}
    />,
  );
  expect(screen.getByRole('status')).toHaveTextContent('No supported storage found');
  expect(screen.getAllByText(warning)).toHaveLength(1);
  expect(screen.getByText(offlineWarning)).toBeVisible();
});

it('supports configuring a Swarm Service from its backend summary', () => {
  renderCitadel(
    <ResourceBackupSummary
      source={{ $type: 'SwarmService', swarmServiceId: 'service' }}
      coverage={{ ...coverage, policies: [], volumes: [coverage.volumes[1]] }}
    />,
  );
  expect(screen.getByRole('link', { name: 'Configure backup' }).getAttribute('href')).toContain(
    'swarmServiceId=service',
  );
});

it('explains the unprotected indicator on keyboard focus', async () => {
  const { user } = renderCitadel(
    <CoverageIndicator volumeName="uploads" coverage={{ status: Status.Unprotected, policies: [] }} />,
  );
  await user.tab();
  expect(await screen.findByRole('tooltip')).toHaveTextContent('No backup policy covers this volume.');
});

it('allows a separate bind policy for storage excluded from the workload and requires platform write permission', () => {
  const bindCoverage = {
    ...coverage,
    hasResourcePolicy: true,
    canConfigureBindMount: true,
    volumes: [
      {
        ...coverage.volumes[1],
        name: '/srv/app data',
        hostPath: '/srv/app data',
        dockerNodeId: 'node-a',
        includedInResourcePolicy: false,
      },
    ],
  };
  const { rerender } = renderCitadel(<ResourceBackupSummary source={source} coverage={bindCoverage} />);
  expect(screen.getByRole('status')).toHaveTextContent('0 of 1 storage targets protected');
  expect(screen.getByText('Bind mount')).toBeVisible();
  expect(screen.queryByRole('link', { name: '/srv/app data' })).not.toBeInTheDocument();
  const link = screen.getByRole('link', { name: 'Configure backup for /srv/app data' });
  expect(link.getAttribute('href')).toContain('sourceType=BindMount');
  expect(link.getAttribute('href')).toContain('dockerNodeId=node-a');
  rerender(<ResourceBackupSummary source={source} coverage={{ ...bindCoverage, canConfigureBindMount: false }} />);
  expect(screen.queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
});

it.each([true, false])('offers schedule editing only with write permission (%s)', (canWrite) => {
  const manual = { ...policy, enabled: true, scheduleConfigured: false, canWrite };
  renderCitadel(
    <ResourceBackupSummary
      source={source}
      coverage={{
        ...coverage,
        policies: [manual],
        volumes: [{ ...coverage.volumes[0], status: Status.Warning, policies: [manual] }],
      }}
    />,
    { route: '/stacks/edit/stack' },
  );
  expect(screen.getByRole('button', { name: 'db: Automatic backups not configured' })).toBeVisible();
  expect(screen.getByText('A successful backup exists, but ongoing protection needs attention.')).toBeVisible();
  const link = screen.queryByRole('link', { name: 'Edit schedule' });
  if (canWrite)
    expect(link).toHaveAttribute(
      'href',
      '/backup-policies/edit/policy?returnTo=%2Fstacks%2Fedit%2Fstack&section=schedule#config',
    );
  else expect(link).not.toBeInTheDocument();
});

it('renders typed inventory and scheduling reasons without deriving status from warning text', () => {
  renderCitadel(
    <ResourceBackupSummary
      source={source}
      coverage={{
        ...coverage,
        status: Status.Warning,
        volumes: [],
        warnings: [],
        reasons: [BackupCoverageReason.InventoryIncomplete, BackupCoverageReason.SchedulingUnavailable],
      }}
    />,
  );
  expect(screen.getByRole('status')).toHaveTextContent('Storage inventory incomplete');
  expect(screen.queryByText('No supported storage found')).not.toBeInTheDocument();
  expect(screen.getByText(/Automatic backups are paused/)).toBeVisible();
});
