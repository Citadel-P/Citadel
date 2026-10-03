import { GitRepoForm } from './form';
import { StateIndicator } from '@/components/custom/state-indicator';
import { GitRepoActions } from './actions';
import { RequiredFormComponents, ResourceFormDataHookResult, RequiredFormFields } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import {
  ActivityStatus,
  GitRepositoryStatus,
  AuthorizedGitRepositoryView,
  LatestActivityView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';
import { useGitRepoGroup } from './hooks/useGitRepoGroup';
import { ActivityAlertZone } from '@/components/custom/task-sheet';
import { AlertMessage } from '@/components/custom/alert-message';
import { truncate } from '@/lib/truncate';
import { GitCommitHorizontalIcon } from 'lucide-react';
import { hasCapability } from '@/lib/resource-capabilities';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { GitRepositoryBrowseAction } from '../browser/browser-dialog';

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
            variant="badge"
            value={(resource as AuthorizedGitRepositoryView).status}
            isProcessing={
              (resource as AuthorizedGitRepositoryView).controlState === ResourceControlState.Processing ||
              (resource as AuthorizedGitRepositoryView).status === GitRepositoryStatus.Pending
            }
          />
        );
      },
      ActionButtons: ({ resource }) => {
        return (
          <GenericActionBarButtons
            resource={resource}
            actions={Object.values(GitRepoActions)}
            standaloneActions={[GitRepositoryBrowseAction]}
          />
        );
      },
      Tags: ({ resource }: { resource: AuthorizedGitRepositoryView }) => (
        <ResourceHeaderTagsEditor
          resourceType="GitRepository"
          resourceId={resource.id}
          tags={resource.tags}
          disabled={!hasCapability(resource, 'canWrite')}
        />
      ),
    },
    SubHeader: ({ resource }: { resource: AuthorizedGitRepositoryView }) => (
      <GitRepoSubHeader latestActivity={resource.latestActivityView} status={resource.status} />
    ),
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }: { resource: AuthorizedGitRepositoryView; metadataChanged?: boolean }) => {
          return (
            <GitRepoForm
              mode="edit"
              metadataChanged={metadataChanged}
              disabled={!hasCapability(resource, 'canWrite')}
            />
          );
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: AuthorizedGitRepositoryView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="GitRepository" />;
        },
      },
    ],
    useData: function (id?: string): ResourceFormDataHookResult {
      const { gitRepo, isLoading, error, refetch, isFetching } = useGitRepoGroup(id);
      return { error, refetch, isFetching, item: gitRepo as any, isLoading };
    },
  },
};

function GitRepoSubHeader({
  latestActivity,
  status,
}: {
  latestActivity: LatestActivityView | null;
  status: GitRepositoryStatus;
}) {
  const info = latestActivity?.info;
  if (!info || (info.$type !== 'GitRepoCloned' && info.$type !== 'GitRepoPulled')) return null;
  // Activity history can retain an earlier failure after the repository recovers.
  if (status === GitRepositoryStatus.Healthy && latestActivity.status === ActivityStatus.Failure) return null;

  if (latestActivity.status === ActivityStatus.Success) {
    const commitSha = info.result?.commitSha;
    if (!commitSha) return null;

    return (
      <AlertMessage date={latestActivity.createdAt} type={'success'}>
        <div className="flex flex-wrap gap-2 items-center ">
          Repository updated successfully to commit <GitCommitHorizontalIcon width={13} height={13} />
          {truncate(commitSha, 12, 'right', true)}
        </div>
      </AlertMessage>
    );
  }
  return (
    <ActivityAlertZone
      info={info}
      activity={latestActivity as any}
      title="Sync Error"
      date={latestActivity?.createdAt}
    />
  );
}
