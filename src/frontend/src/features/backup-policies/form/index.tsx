import { BackupPolicyView, ResourceControlState } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { ActivitiesTab } from '@/features/activities';
import { hasCapability } from '@/lib/resource-capabilities';
import { useRead } from '@/lib/hooks';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { BackupPolicyInfoActions } from '../actions';
import { BackupPolicyForm } from './form';
import { BackupPolicyRunsTab } from './runs';

type BackupPolicyFormResource = BackupPolicyView & RequiredFormFields;

export const BackupPolicyFormComponents: RequiredFormComponents<BackupPolicyFormResource> = {
  AddForm: {
    Header: {
      title: 'Backup Policy',
    },
    Content: () => <BackupPolicyForm mode="add" />,
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }) => {
        const policy = resource as BackupPolicyView;
        return (
          <StateIndicator
            variant="badge"
            value={policy.enabled}
            isProcessing={policy.controlState === ResourceControlState.Processing}
            enableLabel
          />
        );
      },
      ActionButtons: ({ resource }) => {
        const { edit: _edit, ...actions } = BackupPolicyInfoActions;
        return <GenericActionBarButtons resource={resource} actions={Object.values(actions)} />;
      },
      Tags: ({ resource }) => (
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <ResourceHeaderTagsEditor
            resourceType="BackupPolicy"
            resourceId={(resource as BackupPolicyView).id}
            tags={(resource as BackupPolicyView).tags}
            disabled={!hasCapability(resource, 'canWrite')}
          />
        </div>
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }) => (
          <BackupPolicyForm
            mode="edit"
            resource={resource as BackupPolicyView}
            disabled={!hasCapability(resource, 'canWrite')}
            metadataChanged={metadataChanged}
          />
        ),
      },
      {
        label: 'Runs',
        Content: ({ resource }) => <BackupPolicyRunsTab resource={resource as BackupPolicyView} />,
      },
      {
        label: 'Activities',
        Content: ({ resource }) => (
          <ActivitiesTab resourceId={(resource as BackupPolicyView).id} resourceType="BackupPolicy" />
        ),
      },
    ],
    useData(id: string) {
      const { data, isLoading, error, refetch, isFetching } = useRead('getBackupPolicy', { id });
      return { error, refetch, isFetching, item: data?.data as BackupPolicyFormResource | undefined, isLoading };
    },
  },
};
