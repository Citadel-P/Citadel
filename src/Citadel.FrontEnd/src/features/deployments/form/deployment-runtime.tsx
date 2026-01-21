import { DeploymentStatus, DeploymentView } from '@/api/generated/api.types';
import { ContainerInfoTable } from '@/features/docker-resources/containers/container-info/container-info-table';
import { useContainerInfoGroup } from '@/features/docker-resources/containers/hooks/useContainerInfoGroup';
import Loader from '@/components/ui/loader';

export const DeploymentRuntime = ({ deployment }: { deployment: DeploymentView }) => {
  const { containerInfo, isLoading } = useContainerInfoGroup(
    deployment.dockerContainerId ?? undefined,
    deployment.platformId,
  );

  if (isLoading && deployment.status !== DeploymentStatus.Created) return <Loader />;
  if (!containerInfo) return null;
  return (
    <ContainerInfoTable
      container={containerInfo}
      displayOptions={{
        DisplayContainerName: true,
        DisplayStatus: false,
      }}
    />
  );
};
