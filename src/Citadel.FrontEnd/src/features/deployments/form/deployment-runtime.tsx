import { DeploymentView } from '@/api/generated/api.types';
import { ContainerInfoTable } from '@/features/docker-resources/containers/container-info/container-info-table';
import { useContainerInfoGroup } from '@/features/docker-resources/containers/hooks/useContainerInfoGroup';
import Loader from '@/components/ui/loader';

export const DeploymentRuntime = ({ deployment }: { deployment: DeploymentView }) => {
  const { containerInfo, isLoading } = useContainerInfoGroup(
    deployment.dockerContainerId ?? undefined,
    deployment.platformId,
  );
  if (isLoading) return <Loader />;
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
