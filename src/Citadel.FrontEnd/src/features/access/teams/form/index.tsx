import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { TeamForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useRead } from '@/lib/hooks';
import { TeamView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { TeamActions } from './actions';
import { hasCapability } from '@/lib/resource-capabilities';

export const TeamFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'Team',
    },
    Content: () => <TeamForm mode="add" />,
  },
  EditForm: {
    skipMetadataUpdate: true,
    Header: {
      canEditDescription: false,
      Indicator: ({ resource }: { resource: TeamView }) => (
        <StateIndicator value={resource.isEnabled as any} enableLabel={true} />
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
    ],

    useData: function (id?: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { data, isLoading } = useRead('getTeam', { id });
      return { item: data?.data as any, isLoading };
    },
  },
};
