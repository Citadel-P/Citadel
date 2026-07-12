import { BackupRepositoryView } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { BackupRepositoryDropdownActions, BackupRepositoryGroupActions } from './actions';
import { BackupRepositoriesTable } from './table';

const EMPTY_REPOSITORIES: BackupRepositoryView[] = [];

export const BackupRepositoryComponents: RequiredComponents<BackupRepositoryView> = {
  Icon: CitadelIcons.BackupRepository,
  Content: ({ items, actions, isLoading }) => (
    <BackupRepositoriesTable items={items} actions={actions} isLoading={isLoading} />
  ),
  header: {
    title: 'Backup Repositories',
    subtitle: 'Configure encrypted destinations used by backup policies and maintenance operations.',
    showSearch: true,
    showAdd: true,
  },
  DropdownActions: BackupRepositoryDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="BackupRepository" items={items} actions={Object.values(BackupRepositoryGroupActions)} />
  ),
  useData(): ResourceDataHookResult<BackupRepositoryView> {
    const { data, isLoading } = useRead('listBackupRepositories');
    return {
      items: data?.data.repositories ?? EMPTY_REPOSITORIES,
      isLoading,
      capabilities: data?.data.capabilities,
    };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const value = search.toLowerCase();
    return items.filter(
      (repository) =>
        repository.name.toLowerCase().includes(value) ||
        repository.description?.toLowerCase().includes(value) ||
        repository.type.toLowerCase().includes(value) ||
        JSON.stringify(repository.spec).toLowerCase().includes(value),
    );
  },
};
