import { BackupRepositoryView, ResourceCapabilities } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { HubConnection } from '@microsoft/signalr';
import { useCallback, useEffect, useRef, useState } from 'react';

export const useBackupRepositoriesGroup = () => {
  const { data, isLoading } = useRead('listBackupRepositories');
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
    (hubConnection: HubConnection) => {
      hubConnection.on('BackupRepositoryInfoUpdated', handleBackupRepositoryInfoUpdated);
    },
    [handleBackupRepositoryInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('BackupRepositoryInfoUpdated', handleBackupRepositoryInfoUpdated);
    },
    [handleBackupRepositoryInfoUpdated],
  );

  useSignalRGroup({
    groupName: 'backup-repositories',
    setupEventListeners,
    removeEventListeners,
  });

  return { repositories, isLoading, capabilities };
};
