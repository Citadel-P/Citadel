import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { DeploymentForm } from './form';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { DeploymentActions } from './actions';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useDeploymentGroup } from './hooks/useDeploymentGroup';
import { ContainerLogs } from '@/features/docker-resources/containers/container-info/container-logs';
import { DeploymentRuntime } from './deployment-runtime';
import { DeploymentView, ResourceControlState } from '@/api/generated/api.types';
import { normalizeDockerId } from '@/lib/utils';

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
    SubHeader: ({ resource }: { resource: DeploymentView }) => {
      return <DeploymentRuntime key={resource.containerId} deployment={resource} />;
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ metadataChanged }: { metadataChanged?: boolean }) => {
          return <DeploymentForm mode="edit" metadataChanged={metadataChanged} />;
        },
      },
      {
        label: 'Logs',
        Content: ({ resource }: { resource: DeploymentView }) => {
          const nid = normalizeDockerId(resource?.dockerContainerId ?? undefined);
          return <ContainerLogs key={nid} containerId={nid} />;
        },
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { deployment, isLoading } = useDeploymentGroup(id);
      return { item: deployment, isLoading };
    },
  },
};
