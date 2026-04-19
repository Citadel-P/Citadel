import { GitRepoForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { GitRepoActions } from './actions';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import {
  ActivityStatus,
  GitRepositoryView,
  LatestActivityView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';
import { useGitRepoGroup } from './hooks/useGitRepoGroup';
import { ActivityAlertZone } from '@/components/custom/task-sheet';
import { AlertMessage } from '@/components/custom/alert-message';
import { truncate } from '@/lib/truncate';
import { GitCommitHorizontalIcon } from 'lucide-react';

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
    SubHeader: ({ resource }: { resource: GitRepositoryView }) => (
      <GitRepoSubHeader latestActivity={resource.latestActivityView} />
    ),
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

function GitRepoSubHeader({ latestActivity }: { latestActivity: LatestActivityView | null }) {
  if (latestActivity?.status === ActivityStatus.Success && latestActivity.info) {
    return (
      <AlertMessage date={latestActivity?.createdAt} type={'success'}>
        <div className="flex flex-wrap gap-2 items-center ">
          Repository updated successfully to commit <GitCommitHorizontalIcon width={13} height={13} />
          {truncate((latestActivity.info as any).result.commitSha, 12, 'right', true)}
        </div>
      </AlertMessage>
    );
  }
  return (
    <ActivityAlertZone
      info={latestActivity?.info as any}
      activity={latestActivity as any}
      title="Sync Error"
      date={latestActivity?.createdAt}
    />
  );
}
