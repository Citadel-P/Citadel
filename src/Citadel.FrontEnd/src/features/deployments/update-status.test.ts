import { AutoUpdateStatus, DeploymentView } from '@/api/generated/api.types';
import { hasDeploymentUpdateAvailable } from './update-status';

describe('hasDeploymentUpdateAvailable', () => {
  it.each([
    [AutoUpdateStatus.UpdateAvailable, true],
    [AutoUpdateStatus.Unknown, false],
    [AutoUpdateStatus.UpToDate, false],
    [AutoUpdateStatus.Failed, false],
  ])('maps %s to %s', (status, expected) => {
    const deployment = {
      autoUpdateState: { status },
    } as DeploymentView;

    expect(hasDeploymentUpdateAvailable(deployment)).toBe(expected);
  });

  it('is null-safe', () => {
    expect(hasDeploymentUpdateAvailable(undefined)).toBe(false);
  });
});
