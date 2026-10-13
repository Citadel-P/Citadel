import { screen } from '@testing-library/react';
import { BackupSourcePreview, PlatformStatus } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { DeploymentSourcePreview } from './form';

const preview = {
  deploymentId: 'deployment',
  platformId: 'platform',
  platformName: 'Docker',
  platformStatus: PlatformStatus.Online,
  deploymentName: 'App',
  volumes: [
    { name: 'app-data', kind: 'DeclaredNamed', hasBackupCoverage: true, isExternal: false, isShared: false },
    {
      name: 'generated-runtime-name',
      kind: 'AnonymousNamed',
      hasBackupCoverage: true,
      isExternal: false,
      isShared: false,
    },
    { name: '/srv/app', kind: 'BindMount', hasBackupCoverage: false, isExternal: false, isShared: false },
  ],
  warnings: [],
} satisfies BackupSourcePreview;

it('previews only volumes the deployment policy includes and distinguishes policy existence from protection', () => {
  renderCitadel(<DeploymentSourcePreview preview={preview} isLoading={false} hasSelection />);
  expect(screen.getByText('app-data')).toBeVisible();
  expect(screen.getByText('/srv/app')).toBeVisible();
  expect(screen.getByText('Bind mount')).toBeVisible();
  expect(screen.queryByText('generated-runtime-name')).not.toBeInTheDocument();
  expect(screen.getByText(/1 Docker volume is not included/)).toBeVisible();
  expect(screen.getByText('Volume policy configured')).toBeVisible();
  expect(screen.queryByText('Protected')).not.toBeInTheDocument();
});

it('explains when runtime volumes leave a deployment policy with no eligible data', () => {
  renderCitadel(
    <DeploymentSourcePreview preview={{ ...preview, volumes: [preview.volumes[1]] }} isLoading={false} hasSelection />,
  );
  expect(screen.getByText('No supported volumes or bind mounts were resolved for this deployment.')).toBeVisible();
  expect(screen.getByText(/Configure a separate Docker volume policy/)).toBeVisible();
});
