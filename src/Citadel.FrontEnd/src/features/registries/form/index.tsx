import { RegistryForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RegistryActions } from './actions';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { RegistryView } from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';

export const RegistryFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <RegistryForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return <StateIndicator value={resource.status as any} />;
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(RegistryActions)} />;
      },
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => {
          return <RegistryForm mode="edit" resource={resource} />;
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: RegistryView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="Registry" />;
        },
      },
    ],
    useData: function (id?: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { data, isLoading } = useRead('getRegistryConfig', { id });
      return { item: data?.data, isLoading };
    },
  },
};
