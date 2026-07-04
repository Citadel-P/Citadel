import { useEffect, useState, useCallback, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { GitRepositoryView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';

export const useGitReposGroup = () => {
  const { selectedTagIds } = useResourceTagFilter();
  const { data, isLoading } = useRead(
    'listGitRepositories',
    selectedTagIds.length > 0 ? { query: { tagIds: selectedTagIds } } : undefined,
  );
  const [gitRepos, setGitRepos] = useState<GitRepositoryView[] | undefined>();
  const lastFetchedRef = useRef<GitRepositoryView[] | undefined>(data?.data?.gitRepositories);
  const capabilities = data?.data.capabilities;

  useEffect(() => {
    if (data?.data?.gitRepositories && data.data.gitRepositories !== lastFetchedRef.current) {
      lastFetchedRef.current = data.data.gitRepositories;
      setGitRepos(data.data.gitRepositories);
    }
  }, [data?.data?.gitRepositories]);

  const handleGitRepoInfoUpdated = useCallback((repo: GitRepositoryView, action: string) => {
    setGitRepos((prev) => {
      if (!prev) return prev;

      if (action === 'create') {
        return [...prev, repo];
      }
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

  return { gitRepos, capabilities, isLoading, selectedTagIds };
};
