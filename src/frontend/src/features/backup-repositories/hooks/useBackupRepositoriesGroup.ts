import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useRead } from '@/lib/hooks';
import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';

export const useBackupRepositoriesGroup = () => {
  const groupName = 'backup-repositories';
  const structuralSharing = useCollectionStructuralSharing(groupName);
  const query = useRead('listBackupRepositories', undefined, {
    structuralSharing,
  });
  const repositories = useCollectionSubscription({
    groupName,
    eventName: 'BackupRepositoryInfoUpdated',
    field: 'repositories',
    query,
  });
  const { data, error, refetch, isFetching, isLoading } = query;
  return { error, refetch, isFetching, repositories, isLoading, capabilities: data?.data.capabilities };
};
