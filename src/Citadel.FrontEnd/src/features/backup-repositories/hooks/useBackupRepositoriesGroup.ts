import { BackupRepositoryView, ResourceCapabilities } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useEffect, useRef, useState } from 'react';

export const useBackupRepositoriesGroup = () => {
  const { data, isLoading, error, refetch, isFetching } = useRead('listBackupRepositories');
  const [repositories, setRepositories] = useState<BackupRepositoryView[] | undefined>();
  const [capabilities, setCapabilities] = useState<ResourceCapabilities | undefined>();
  const lastFetchedRef = useRef<BackupRepositoryView[]>([]);

  useEffect(() => {
    if (!data) return;

    const newBase = data.data.repositories;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setRepositories(newBase);
      setCapabilities(data.data.capabilities);
    }
  }, [data]);

  const handleBackupRepositoryInfoUpdated = useCallback((repository: BackupRepositoryView, action: string) => {
    setRepositories((prev) => {
      if (!prev) return prev;
      if (action === 'create') return [...prev, repository];
      if (action === 'delete') return prev.filter((item) => item.id !== repository.id);

      const index = prev.findIndex((item) => item.id === repository.id);
      if (index === -1) return [...prev, repository];

      const updated = [...prev];
      updated[index] = repository;
      return updated;
    });
  }, []);

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
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, repositories, isLoading, capabilities };
};
