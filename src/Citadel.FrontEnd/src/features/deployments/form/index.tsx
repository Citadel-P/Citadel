import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { DeploymentForm } from './form';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { DeploymentActions } from './actions';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useDeploymentGroup } from './hooks/useDeploymentGroup';
import { ContainerStateStatus, DeploymentStatus, DeploymentView, ResourceControlState } from '@/api/generated/api.types';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ContainerLogs } from '@/features/docker-resources/containers/container-info/container-logs';
import { DockerContainerView } from '@/api/types';
import ContainerInspect from '@/features/docker-resources/containers/container-info/container-inspect';
import { normalizeDockerId } from '@/lib/utils';
import { ContainerInfoTable } from '@/features/docker-resources/containers/container-info/container-info-table';
import { useContainerInfoGroup } from '@/features/docker-resources/containers/hooks/useContainerInfoGroup';
import Loader from '@/components/ui/loader';
import { ContainerExec } from '@/features/docker-resources/containers/container-info/container-exec';

export const DeploymentFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <DeploymentForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return (
          <StateIndicator
            value={resource.status as any}
            isProcessing={(resource as any).controlState === ResourceControlState.Processing}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(DeploymentActions)} />;
      },
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ metadataChanged }: { metadataChanged?: boolean }) => {
          return <DeploymentForm mode="edit" metadataChanged={metadataChanged} />;
        },
      },
      {
        label: 'Container',
        disabled: (resource: DeploymentView): boolean => resource.status === DeploymentStatus.Degraded,
        Content: ({ resource }: { resource: DeploymentView }) => {
          return <DeploymentRuntime key={resource.containerId} deployment={resource} />;
        },
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { deployment, isLoading } = useDeploymentGroup(id);
      return { item: deployment, isLoading };
    },
  },
};

const DeploymentRuntime = ({ deployment }: { deployment: DeploymentView }) => {
  const { containerInfo, isLoading } = useContainerInfoGroup(
    deployment.dockerContainerId ?? undefined,
    deployment.platformId,
  );

  if (deployment.status === DeploymentStatus.Degraded) return null;

  if (isLoading || !containerInfo) return <Loader />;

  return <RuntimeView containerInfo={containerInfo} />;
};

const RuntimeView = ({ containerInfo }: { containerInfo: DockerContainerView }) => {
  return (
    <div className="flex flex-col gap-4 w-full">
      <ContainerInfoTable
        container={containerInfo}
        displayOptions={{
          DisplayContainerName: true,
          DisplayStatus: false,
        }}
      />
      <RuntimeTabs containerId={containerInfo.id} disabled={containerInfo.state !== ContainerStateStatus.Running}/>
    </div>
  );
};

const RuntimeTabs = ({ containerId, disabled }: { containerId: string, disabled?: boolean}) => {
  const nid = normalizeDockerId(containerId);

  return (
    <Tabs defaultValue="logs" className="w-full">
      <TabsList className="w-fit justify-start">
        <TabsTrigger className="text-xs" value="logs">
          Logs
        </TabsTrigger>
        <TabsTrigger className="text-xs" value="inspect">
          Inspect
        </TabsTrigger>
        <TabsTrigger className="text-xs" value="terminal" disabled={disabled}>
          Terminal
        </TabsTrigger>
      </TabsList>
      <TabsContent value="logs" className="w-full mt-2">
        <ContainerLogs key={nid} containerId={nid} />
      </TabsContent>
      <TabsContent value="inspect" className="w-full mt-2">
        <ContainerInspect key={nid} containerId={nid} />
      </TabsContent>
      <TabsContent value="terminal" className="w-full mt-2" >
        <ContainerExec key={nid} containerId={nid} disabled={disabled} />
      </TabsContent>
    </Tabs>
  );
};
