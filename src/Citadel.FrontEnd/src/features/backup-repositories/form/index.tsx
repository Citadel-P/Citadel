import { BackupRepositoryView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { hasCapability } from '@/lib/resource-capabilities';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { useRead } from '@/lib/hooks';
import { BackupRepositoryInfoActions } from '../actions';
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
      Indicator: ({ resource }: { resource: RequiredFormFields }) => (
        <StateIndicator value={(resource as BackupRepositoryView).status} />
      ),
      ActionButtons: ({ resource }) => (
        <GenericActionBarButtons resource={resource} actions={Object.values(BackupRepositoryInfoActions)} />
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }: { resource: BackupRepositoryView }) => (
          <BackupRepositoryForm mode="edit" resource={resource} disabled={!hasCapability(resource, 'canWrite')} />
        ),
      },
    ],
    useData(id?: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { data, isLoading } = useRead('getBackupRepository', { id });
      return { item: data?.data, isLoading };
    },
    skipMetadataUpdate: true,
  },
};
