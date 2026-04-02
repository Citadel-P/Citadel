import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { GitRepositoryView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useGitReposGroup = () => {
  const { data, isLoading } = useRead('listGitRepositories');
  const [gitRepos, setGitRepos] = useState<GitRepositoryView[] | undefined>();

  useEffect(() => {
    if (!data) return;
    setGitRepos(data.data.gitRepositories);
  }, [data]);

  const handleGitRepoInfoUpdated = useCallback((repo: GitRepositoryView, action: string) => {
    setGitRepos((prev) => {
      if (!prev) return prev;

      if (action === 'delete') {
        return prev.filter((d) => d.id !== repo.id);
      }

      const index = prev.findIndex((d) => d.id === repo.id);
      if (index !== -1) {
        const updated = [...prev];
        updated[index] = repo;
        return updated;
      }
      return prev;
    });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('GitRepositoryInfoUpdated', handleGitRepoInfoUpdated);
    },
    [handleGitRepoInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('GitRepositoryInfoUpdated', handleGitRepoInfoUpdated);
    },
    [handleGitRepoInfoUpdated],
  );

  useSignalRGroup({
    groupName: 'git-repositories',
    setupEventListeners,
    removeEventListeners,
  });

  return { gitRepos, isLoading };
};
