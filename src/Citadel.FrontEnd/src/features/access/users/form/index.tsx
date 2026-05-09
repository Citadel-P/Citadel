import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { UserForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';
import { UserView } from '@/api/generated/api.types';

export const UserFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'User',
    },
    Content: () => <UserForm mode="add" />,
  },
  EditForm: {
    skipMetadataUpdate: true,
    Header: {
      canEditDescription: false,
      Indicator: ({ resource }: { resource: UserView }) => {
        return <StateIndicator value={resource.isEnabled as any} enableLabel={true} />;
      },
      ActionButtons: () => null,
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
      const { data, isLoading } = useRead('getUser', { id });
      return { item: data?.data as any, isLoading };
    },
  },
};
