import { useEffect, useState, useCallback, useRef } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { GitRepositoryView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
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
    const info = (repo.latestActivityView?.info as any)?.[1]; // realtime poly mapping
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
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('GitRepositoryInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('GitRepositoryInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'git-repo:' + id,
    skip: !id,
    setupEventListeners,
    removeEventListeners,
  });

  return { gitRepo, isLoading };
};
