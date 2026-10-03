import {
  AutoUpdateStatus,
  DeploymentStatus,
  DeploymentView,
  ResourceControlState,
  UpdateBehavior,
} from '@/api/generated/api.types';
import {
  canCheckDeploymentUpdates,
  getDeploymentUpdateCheckDisabledReason,
  hasDeploymentUpdateAvailable,
} from './update-status';

describe('deployment update status', () => {
  it('does not allow update checks before the deployment is applied', () => {
    expect(canCheckDeploymentUpdates({ status: DeploymentStatus.Created } as DeploymentView)).toBe(false);
    expect(canCheckDeploymentUpdates(checkableDeployment())).toBe(true);
    expect(
      canCheckDeploymentUpdates({
        ...checkableDeployment(),
        controlState: ResourceControlState.Processing,
      }),
    ).toBe(false);
  });

  it('explains why local and build images cannot be checked', () => {
    expect(
      getDeploymentUpdateCheckDisabledReason({
        ...checkableDeployment(),
        spec: {
          image: { $type: 'Local', imageId: 'image-id' },
          updateBehavior: UpdateBehavior.Disabled,
        },
      } as DeploymentView),
    ).toBe('Update checks are only available for external tagged images.');
    expect(
      getDeploymentUpdateCheckDisabledReason({
        ...checkableDeployment(),
        spec: {
          image: { $type: 'Build', buildProjectId: 'build-id' },
          updateBehavior: UpdateBehavior.Disabled,
        },
      } as DeploymentView),
    ).toBe('Update checks are only available for external tagged images.');
  });

  it('requires a tagged external reference and an applied digest', () => {
    expect(
      getDeploymentUpdateCheckDisabledReason(
        checkableDeployment({
          registryId: '00000000-0000-0000-0000-000000000000',
        }),
      ),
    ).toContain('registry');
    expect(
      getDeploymentUpdateCheckDisabledReason(
        checkableDeployment({
          imageTag: 'nginx@sha256:abc',
        }),
      ),
    ).toContain('Digest-pinned images');
    expect(
      getDeploymentUpdateCheckDisabledReason(
        checkableDeployment({
          resolvedDigest: null,
        }),
      ),
    ).toContain('applied digest');
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

const checkableDeployment = (
  image: Partial<{
    registryId: string;
    imageTag: string;
    resolvedDigest: string | null;
  }> = {},
): DeploymentView =>
  ({
    status: DeploymentStatus.Healthy,
    controlState: ResourceControlState.Idle,
    spec: {
      image: {
        $type: 'External',
        registryId: '019fb000-0000-7000-8000-000000000001',
        imageTag: 'nginx:latest',
        resolvedDigest: 'sha256:current',
        ...image,
      },
      updateBehavior: UpdateBehavior.Disabled,
    },
  }) as DeploymentView;
