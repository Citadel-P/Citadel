import { BackupPolicyView, ResourceControlState } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TagChips } from '@/features/tags/components';
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
      canEditTitle: false,
      canEditDescription: false,
      Indicator: ({ resource }) => {
        const policy = resource as BackupPolicyView;
        return (
          <StateIndicator
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
          <TagChips tags={(resource as BackupPolicyView).tags} />
        </div>
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => (
          <BackupPolicyForm
            mode="edit"
            resource={resource as BackupPolicyView}
            disabled={!hasCapability(resource, 'canWrite')}
          />
        ),
      },
      {
        label: 'Runs',
        Content: ({ resource }) => <BackupPolicyRunsTab resource={resource as BackupPolicyView} />,
      },
    ],
    useData(id: string) {
      const { data, isLoading } = useRead('getBackupPolicy', { id });
      return {
        item: data?.data as BackupPolicyFormResource | undefined,
        isLoading,
      };
    },
    skipMetadataUpdate: true,
  },
};
