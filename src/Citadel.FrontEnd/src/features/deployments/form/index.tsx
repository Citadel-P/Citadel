import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { DeploymentForm } from './form';
import { useRead } from '@/lib/hooks';

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
        return <></>;
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
