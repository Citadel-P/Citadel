import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback } from 'react';
import { useCollectionRows } from '@/hooks/useCollectionRows';

const allRepositories = () => true;

export const useBackupRepositoriesGroup = () => {
  const { data, isLoading, error, refetch, isFetching } = useRead('listBackupRepositories');
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.repositories : undefined;
  }, [refetch]);
  const [repositories, handleBackupRepositoryInfoUpdated, onJoinedGroup] = useCollectionRows(
    data?.data.repositories,
    'backup-repositories',
    allRepositories,
    reload,
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('BackupRepositoryInfoUpdated', handleBackupRepositoryInfoUpdated);
    },
    [handleBackupRepositoryInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('BackupRepositoryInfoUpdated', handleBackupRepositoryInfoUpdated);
    },
    [handleBackupRepositoryInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'backup-repositories',
    onJoinedGroup,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, repositories, isLoading, capabilities: data?.data.capabilities };
};
