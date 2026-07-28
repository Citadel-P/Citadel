import { AutoUpdateStatus, DeploymentStatus, DeploymentView, ResourceControlState } from '@/api/generated/api.types';
import { canCheckDeploymentUpdates, hasDeploymentUpdateAvailable } from './update-status';

describe('hasDeploymentUpdateAvailable', () => {
  it('does not allow update checks before the deployment is applied', () => {
    expect(canCheckDeploymentUpdates({ status: DeploymentStatus.Created } as DeploymentView)).toBe(false);
    expect(canCheckDeploymentUpdates({ status: DeploymentStatus.Healthy } as DeploymentView)).toBe(true);
    expect(
      canCheckDeploymentUpdates({
        status: DeploymentStatus.Healthy,
        controlState: ResourceControlState.Processing,
      } as DeploymentView),
    ).toBe(false);
  });

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
