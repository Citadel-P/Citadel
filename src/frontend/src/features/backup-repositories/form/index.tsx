import { BackupRepositoryView, BackupRepositoryStatus } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredFormComponents, ResourceFormDataHookResult, RequiredFormFields } from '@/pages/types';
import { useBackupRepositoryGroup } from '../hooks/useBackupRepositoryGroup';
import { BackupRepositoryInfoActions } from '../actions';
import { RepositoryCredentials } from '../credentials';
import { BackupRepositoryForm } from './form';

export const BackupRepositoryFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: {
      title: 'Backup Repository',
    },
    Content: () => <BackupRepositoryForm mode="add" />,
  },
  EditForm: {
    Header: {
      canEditTitle: false,
      canEditDescription: false,
      showDescription: false,
      Indicator: ({ resource }: { resource: RequiredFormFields }) => (
        <StateIndicator variant="badge" value={(resource as BackupRepositoryView).status} />
      ),
      ActionButtons: ({ resource }) => (
        <GenericActionBarButtons resource={resource} actions={Object.values(BackupRepositoryInfoActions)} />
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }: { resource: BackupRepositoryView }) => (
          <div className="space-y-6">
            <BackupRepositoryForm mode="edit" resource={resource} disabled={resource.capabilities?.canWrite !== true} />
            {resource.status === BackupRepositoryStatus.Ready && resource.spec.$type === 'S3Compatible' && (
              <RepositoryCredentials key={resource.id} repository={resource} />
            )}
          </div>
        ),
      },
    ],
    useData(id: string): ResourceFormDataHookResult {
      const { data, isLoading, error, refetch, isFetching } = useBackupRepositoryGroup(id);
      return { error, refetch, isFetching, item: data?.data, isLoading };
    },
    skipMetadataUpdate: true,
  },
};
