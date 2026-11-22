import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useState, useCallback, useRef } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { AuthorizedGitRepositoryView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';

export const useGitRepoGroup = (id: string | undefined) => {
  const client = useQueryClient();
  const { data, isLoading, error, refetch, isFetching } = useRead('getGitRepository', { id });
  const [gitRepo, setGitRepo] = useState<AuthorizedGitRepositoryView | undefined>(data?.data);
  const lastDataRef = useRef<AuthorizedGitRepositoryView | undefined>(data?.data);

  useEffect(() => {
    if (data?.data && data.data !== lastDataRef.current) {
      lastDataRef.current = data.data;
      setGitRepo(data.data);
    }
  }, [data?.data]);

  const handleGitRepositoryInfoUpdated = useCallback(
    (repo: AuthorizedGitRepositoryView, action: string) => {
      if (repo.id !== id) return;
      if (action === 'delete') {
        const queryKey = ['getGitRepository', { id }];
        void client.cancelQueries({ queryKey, exact: true });
        client.setQueryData(queryKey, null);
        lastDataRef.current = undefined;
        setGitRepo(undefined);
        return;
      }
      const activityInfo = repo.latestActivityView?.info;
      const info = Array.isArray(activityInfo) ? { ...activityInfo[1], $type: activityInfo[0] } : activityInfo;

      setGitRepo((prev) => ({
        ...prev,
        ...repo,
        capabilities: repo.capabilities ?? prev?.capabilities,
        latestActivityView: repo.latestActivityView ? { ...repo.latestActivityView, info } : null,
      }));
    },
    [id, client],
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
