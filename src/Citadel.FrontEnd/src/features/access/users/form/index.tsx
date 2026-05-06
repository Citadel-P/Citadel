import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { UserForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';

export const UserFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'User',
    },
    Content: () => <UserForm mode="add" />,
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return <StateIndicator value={resource.status as any} />;
      },
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => {
          return <UserForm mode="edit" resource={resource} />;
        },
      },
    ],
    useData: function (id?: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { data, isLoading } = useRead('getRegistryConfig', { id });
      return { item: data?.data, isLoading };
    },
  },
};
