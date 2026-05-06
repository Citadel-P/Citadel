import { useEffect, useState, useCallback, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { GitRepositoryView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useGitRepoGroup = (id: string | undefined) => {
  const { data, isLoading } = useRead('getGitRepository', { id });
  const [gitRepo, setGitRepo] = useState<GitRepositoryView | undefined>(data?.data);
  const lastDataRef = useRef<GitRepositoryView | undefined>(data?.data);

  useEffect(() => {
    if (data?.data && data.data !== lastDataRef.current) {
      lastDataRef.current = data.data;
      setGitRepo(data.data);
    }
  }, [data?.data]);

  const handleDeploymentInfoUpdated = useCallback((repo: GitRepositoryView) => {
    const info = (repo.latestActivityView?.info as any)?.[1]; // SignalR poly mapping
    if (info) info.$type = (repo.latestActivityView?.info as any)?.[0];

    setGitRepo((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        ...repo,
        latestActivityView: repo.latestActivityView ? { ...repo.latestActivityView, info } : null,
      };
    });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('GitRepositoryInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('GitRepositoryInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  useSignalRGroup({
    groupName: 'git-repo:' + id,
    skip: !id,
    setupEventListeners,
    removeEventListeners,
  });

  return { gitRepo, isLoading };
};
