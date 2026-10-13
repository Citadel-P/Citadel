import { screen, within } from '@testing-library/react';
import { BackupCoverageStatus, ResourceBackupSummaryView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { BindMountBackupLinks, VolumeBackupLinks } from './volume-backup-links';

const coverage = {
  platformId: 'platform',
  canConfigureBindMount: true,
  volumes: ['/srv/data', '/srv/config.json', '/srv/other'].map((hostPath) => ({
    name: hostPath,
    hostPath,
    dockerNodeId: 'worker',
    status: 'Unprotected',
    policies: [],
  })),
} as unknown as ResourceBackupSummaryView;

it('limits inline paths to two and opens a dialog with all paths and node-specific backup links', async () => {
  const { user } = renderCitadel(<BindMountBackupLinks coverage={coverage} />);
  expect(screen.getByText('/srv/data')).toBeVisible();
  expect(screen.getByText('/srv/config.json')).toBeVisible();
  expect(screen.queryByText('/srv/other')).not.toBeInTheDocument();
  expect(screen.queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'View all 3 bind mounts' }));
  const dialog = within(screen.getByRole('dialog', { name: 'Bind mounts' }));
  expect(dialog.getByText('/srv/other')).toBeVisible();
  await user.click(dialog.getByRole('button', { name: 'Backups for /srv/other: Storage unprotected' }));
  const card = within(await screen.findByRole('dialog', { name: 'Backups' }));
  const link = card.getByRole('link', { name: 'Configure backup for /srv/other' });
  expect(link).toHaveAttribute('href', expect.stringContaining('hostPath=%2Fsrv%2Fother'));
  expect(link).toHaveAttribute('href', expect.stringContaining('dockerNodeId=worker'));
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('dialog', { name: 'Backups' })).not.toBeInTheDocument();
  expect(screen.getByRole('dialog', { name: 'Bind mounts' })).toBeVisible();
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

it('needs no popup for two mounts and hides configure links without host permissions', () => {
  renderCitadel(
    <BindMountBackupLinks
      coverage={{ ...coverage, volumes: coverage.volumes.slice(0, 2), canConfigureBindMount: false }}
    />,
  );
  expect(screen.queryByRole('button', { name: /View all/ })).not.toBeInTheDocument();
  expect(screen.queryByRole('link')).not.toBeInTheDocument();
});

const volumeName = '534ec8361d99ab9d06e5da85923683a1e4960a4c969dd92bf9df5bd9c6af';
const volumeCoverage: ResourceBackupSummaryView = {
  ...coverage,
  canConfigure: true,
  volumes: [{ ...coverage.volumes[0], name: volumeName, hostPath: null, status: BackupCoverageStatus.Unprotected }],
};

it('offers backup setup through the shield for an anonymous volume using its full name, node, and return location', async () => {
  const route = '/platforms/platform/containers/forgejo?tab=overview';
  const { user } = renderCitadel(
    <VolumeBackupLinks volumes={[volumeName]} platformId="platform" dockerNodeId="worker" coverage={volumeCoverage} />,
    { route },
  );
  const volumeLink = screen.getByRole('link', { name: volumeName });
  expect(volumeLink).toHaveTextContent(`${volumeName.slice(0, 32)}...`);
  expect(volumeLink).toHaveAttribute('title', volumeName);
  expect(screen.queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: `Backups for ${volumeName}: Volumes unprotected` }));
  const setup = screen.getByRole('link', { name: `Configure backup for ${volumeName}` });
  const url = new URL(setup.getAttribute('href')!, 'http://localhost');
  expect(url.pathname).toBe('/backup-policies/add');
  expect(Object.fromEntries(url.searchParams)).toEqual({
    sourceType: 'DockerVolume',
    platformId: 'platform',
    volumeName,
    dockerNodeId: 'worker',
    returnTo: route,
  });
});

it.each([
  { ...volumeCoverage, canConfigure: false },
  { ...volumeCoverage, unavailable: true },
  { ...volumeCoverage, volumes: [{ ...volumeCoverage.volumes[0], dockerNodeId: 'another-worker' }] },
])('hides volume setup when permission or matching coverage is unavailable', async (summary) => {
  const { user } = renderCitadel(
    <VolumeBackupLinks volumes={[volumeName]} platformId="platform" dockerNodeId="worker" coverage={summary} />,
  );
  await user.click(screen.getByRole('button', { name: new RegExp(`Backups for ${volumeName}:`) }));
  expect(screen.queryByRole('link', { name: `Configure backup for ${volumeName}` })).not.toBeInTheDocument();
});

it('opens the shared backup card for a volume with its exact backup target', async () => {
  const route = '/platforms/platform/containers/forgejo';
  const { user } = renderCitadel(
    <VolumeBackupLinks volumes={[volumeName]} platformId="platform" dockerNodeId="worker" coverage={volumeCoverage} />,
    { route },
  );
  await user.hover(screen.getByRole('button', { name: `Backups for ${volumeName}: Volumes unprotected` }));
  const card = within(await screen.findByRole('dialog', { name: 'Backups' }));
  expect(card.getByRole('status')).toHaveTextContent('0 of 1 volumes protected');
  const link = card.getByRole('link', { name: `Configure backup for ${volumeName}` });
  const query = new URL(link.getAttribute('href')!, 'http://localhost').searchParams;
  expect(Object.fromEntries(query)).toEqual({
    sourceType: 'DockerVolume',
    platformId: 'platform',
    volumeName,
    dockerNodeId: 'worker',
    returnTo: route,
  });
});

it('limits a bind-mount card to that path and respects host backup permissions', async () => {
  const { user, rerender } = renderCitadel(<BindMountBackupLinks coverage={coverage} />);
  await user.click(screen.getByRole('button', { name: 'Backups for /srv/data: Storage unprotected' }));
  const card = within(screen.getByRole('dialog', { name: 'Backups' }));
  expect(card.getByRole('status')).toHaveTextContent('0 of 1 storage targets protected');
  expect(card.queryByText('/srv/config.json')).not.toBeInTheDocument();
  const link = card.getByRole('link', { name: 'Configure backup for /srv/data' });
  expect(link).toHaveAttribute('href', expect.stringContaining('hostPath=%2Fsrv%2Fdata'));
  expect(link).toHaveAttribute('href', expect.stringContaining('dockerNodeId=worker'));
  rerender(<BindMountBackupLinks coverage={{ ...coverage, canConfigureBindMount: false }} />);
  expect(card.queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
});

it('does not show stale protection or setup actions in an unavailable storage card', async () => {
  const { user } = renderCitadel(
    <VolumeBackupLinks
      volumes={[volumeName]}
      platformId="platform"
      dockerNodeId="worker"
      coverage={{ ...volumeCoverage, unavailable: true }}
    />,
  );
  await user.click(screen.getByRole('button', { name: `Backups for ${volumeName}: Coverage unavailable` }));
  const card = within(screen.getByRole('dialog', { name: 'Backups' }));
  expect(card.getByText('Backup coverage is unavailable.')).toBeVisible();
  expect(card.queryByRole('link', { name: /Configure backup/ })).not.toBeInTheDocument();
});
