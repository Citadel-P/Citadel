import { AutoUpdateStatus, DeploymentView } from '@/api/generated/api.types';

export const hasDeploymentUpdateAvailable = (deployment: DeploymentView | null | undefined): boolean =>
  deployment?.autoUpdateState?.status === AutoUpdateStatus.UpdateAvailable;
