import { AutoUpdateStatus, DeploymentStatus, DeploymentView, ResourceControlState } from '@/api/generated/api.types';

export const canCheckDeploymentUpdates = (deployment: DeploymentView | null | undefined): boolean =>
  !!deployment &&
  deployment.status !== DeploymentStatus.Created &&
  deployment.controlState !== ResourceControlState.Processing;

export const hasDeploymentUpdateAvailable = (deployment: DeploymentView | null | undefined): boolean =>
  deployment?.autoUpdateState?.status === AutoUpdateStatus.UpdateAvailable;
