import { RequiredFormComponents, ResourceFormDataHookResult } from '@/pages/types';
import { UserForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';
import { UserView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { UserActions } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';
import { ActivitiesTab } from '@/features/activities';

export const UserFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'User',
    },
    Content: () => <UserForm mode="add" />,
  },
  EditForm: {
    skipMetadataUpdate: true,
    supportsHeaderRename: true,
    Header: {
      canEditDescription: false,
      Indicator: ({ resource }: { resource: UserView }) => (
        <StateIndicator variant="badge" value={resource.isEnabled as any} enableLabel={true} />
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
      {
        label: 'Activities',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="User" />,
      },
    ],

    useData: function (id?: string): ResourceFormDataHookResult {
      const { data, isLoading, error, refetch, isFetching } = useRead('getUser', { id });
      return { error, refetch, isFetching, item: data?.data as any, isLoading };
    },
  },
};
