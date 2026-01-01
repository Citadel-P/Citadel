import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { DeploymentForm } from './form';
import { useRead } from '@/lib/hooks';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { DeploymentActions } from './actions';

export const DeploymentFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <DeploymentForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return <></>;
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(DeploymentActions)} />;
      },
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => {
          return <DeploymentForm mode="edit" resource={resource} />;
        },
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { data, isLoading } = useRead('getDeploymentConfig', { deploymentId: id });
      return { item: data?.data, isLoading };
    },
  },
};
