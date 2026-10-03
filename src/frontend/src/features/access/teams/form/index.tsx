import { RequiredFormComponents, ResourceFormDataHookResult } from '@/pages/types';
import { TeamForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';
import { TeamView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { TeamActions } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';
import { ActivitiesTab } from '@/features/activities';

export const TeamFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'Team',
    },
    Content: () => <TeamForm mode="add" />,
  },
  EditForm: {
    skipMetadataUpdate: true,
    supportsHeaderRename: true,
    Header: {
      canEditDescription: false,
      Indicator: ({ resource }: { resource: TeamView }) => (
        <StateIndicator variant="badge" value={resource.isEnabled as any} enableLabel={true} />
      ),
      ActionButtons: ({ resource }) => (
        <GenericActionBarButtons resource={resource} actions={Object.values(TeamActions)} />
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => (
          <TeamForm mode="edit" resource={resource} disabled={!hasCapability(resource, 'canWrite')} />
        ),
      },
      {
        label: 'Activities',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="Team" />,
      },
    ],

    useData: function (id?: string): ResourceFormDataHookResult {
      const { data, isLoading, error, refetch, isFetching } = useRead('getTeam', { id });
      return { error, refetch, isFetching, item: data?.data as any, isLoading };
    },
  },
};
