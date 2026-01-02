import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { DeploymentForm } from './form';
import { useRead } from '@/lib/hooks';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { DeploymentActions } from './actions';
import { StateIndicator } from '@/components/custom/state-indicator';

export const DeploymentFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <DeploymentForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return <StateIndicator value={resource.status as any} />;
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
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { data, isLoading } = useRead('getDeployment', { deploymentId: id });
      return { item: data?.data, isLoading };
    },
  },
};
