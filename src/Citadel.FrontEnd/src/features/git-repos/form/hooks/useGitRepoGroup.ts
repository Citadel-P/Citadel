import { useEffect, useState, useCallback, useRef } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { GitRepositoryView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';

export const useGitRepoGroup = (id: string | undefined) => {
  const { data, isLoading, error, refetch, isFetching } = useRead('getGitRepository', { id });
  const [gitRepo, setGitRepo] = useState<GitRepositoryView | undefined>(data?.data);
  const lastDataRef = useRef<GitRepositoryView | undefined>(data?.data);

  useEffect(() => {
    if (data?.data && data.data !== lastDataRef.current) {
      lastDataRef.current = data.data;
      setGitRepo(data.data);
    }
  }, [data?.data]);

  const handleGitRepositoryInfoUpdated = useCallback(
    (repo: GitRepositoryView) => {
      if (repo.id !== id) return;
      const activityInfo = repo.latestActivityView?.info;
      const info = Array.isArray(activityInfo) ? { ...activityInfo[1], $type: activityInfo[0] } : activityInfo;

      setGitRepo((prev) => ({
        ...prev,
        ...repo,
        capabilities: repo.capabilities ?? prev?.capabilities,
        latestActivityView: repo.latestActivityView ? { ...repo.latestActivityView, info } : null,
      }));
    },
    [id],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('GitRepositoryInfoUpdated', handleGitRepositoryInfoUpdated);
    },
    [handleGitRepositoryInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('GitRepositoryInfoUpdated', handleGitRepositoryInfoUpdated);
    },
    [handleGitRepositoryInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'git-repo:' + id,
    skip: !id,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, gitRepo, isLoading };
};
