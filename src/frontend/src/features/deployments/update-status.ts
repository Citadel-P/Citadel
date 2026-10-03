import { AutoUpdateStatus, DeploymentStatus, DeploymentView, ResourceControlState } from '@/api/generated/api.types';

export const getDeploymentUpdateCheckDisabledReason = (
  deployment: DeploymentView | null | undefined,
): string | undefined => {
  if (!deployment) return 'Select a deployment to check for updates.';
  if (deployment.status === DeploymentStatus.Created) {
    return 'Apply this deployment before checking for image updates.';
  }
  if (deployment.controlState === ResourceControlState.Processing) {
    return 'Wait for the current deployment operation to finish.';
  }

  const image = deployment.spec?.image;
  if (image?.$type !== 'External') {
    return 'Update checks are only available for external tagged images.';
  }
  if (!image.registryId || image.registryId === '00000000-0000-0000-0000-000000000000') {
    return 'Select a registry before checking for image updates.';
  }
  if (!isTaggedImageReference(image.imageTag)) {
    return 'Use a tagged image reference such as nginx:latest. Digest-pinned images cannot be checked.';
  }
  if (!image.resolvedDigest?.trim()) {
    return 'Redeploy this image once so Citadel has an applied digest to compare.';
  }

  return undefined;
};

export const canCheckDeploymentUpdates = (deployment: DeploymentView | null | undefined): boolean =>
  getDeploymentUpdateCheckDisabledReason(deployment) === undefined;

export const hasDeploymentUpdateAvailable = (deployment: DeploymentView | null | undefined): boolean =>
  deployment?.autoUpdateState?.status === AutoUpdateStatus.UpdateAvailable;

const isTaggedImageReference = (image: string | null | undefined): boolean => {
  const value = image?.trim();
  if (!value || value.includes('@')) return false;

  const lastSlash = value.lastIndexOf('/');
  const lastColon = value.lastIndexOf(':');
  if (lastColon <= lastSlash) return true;

  return value.slice(0, lastColon).trim().length > 0 && value.slice(lastColon + 1).trim().length > 0;
};
