import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { UserForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';
import { UserView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { UserActions } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';

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
      Indicator: ({ resource }: { resource: UserView }) => (
        <StateIndicator value={resource.isEnabled as any} enableLabel={true} />
      ),
      ActionButtons: ({ resource }) => (
        <GenericActionBarButtons resource={resource} actions={Object.values(UserActions)} />
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => (
          <UserForm mode="edit" resource={resource} disabled={!hasCapability(resource, 'canWrite')} />
        ),
      },
    ],

    useData: function (id?: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { data, isLoading } = useRead('getUser', { id });
      return { item: data?.data as any, isLoading };
    },
  },
};
