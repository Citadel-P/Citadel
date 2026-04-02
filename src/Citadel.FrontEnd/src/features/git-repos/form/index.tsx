import { GitRepoForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { GitRepoActions } from './actions';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { GitRepositoryView, ResourceControlState } from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';
import { useGitRepoGroup } from './hooks/useGitRepoGroup';

const title = 'Repository';
export const GitRepoFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title,
    },
    Content: () => {
      return <GitRepoForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return (
          <StateIndicator
            value={(resource as GitRepositoryView).status}
            isProcessing={(resource as GitRepositoryView).controlState === ResourceControlState.Processing}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(GitRepoActions)} />;
      },
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ metadataChanged }) => {
          return <GitRepoForm mode="edit" metadataChanged={metadataChanged} />;
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: GitRepositoryView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="GitRepository" />;
        },
      },
    ],
    useData: function (id?: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { gitRepo, isLoading } = useGitRepoGroup(id);
      return { item: gitRepo as any, isLoading };
    },
  },
};
