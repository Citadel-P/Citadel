import { useEffect, useState, useCallback, useRef } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { AuthorizedGitRepositoryView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';

export const useGitReposGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const { data, isLoading, error, refetch, isFetching } = useRead(
    'listGitRepositories',
    selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined,
  );
  const [gitRepos, setGitRepos] = useState<AuthorizedGitRepositoryView[] | undefined>();
  const lastFetchedRef = useRef<AuthorizedGitRepositoryView[] | undefined>(data?.data?.gitRepositories);
  const capabilities = data?.data.capabilities;

  useEffect(() => {
    if (data?.data?.gitRepositories && data.data.gitRepositories !== lastFetchedRef.current) {
      lastFetchedRef.current = data.data.gitRepositories;
      setGitRepos(data.data.gitRepositories);
    }
  }, [data?.data?.gitRepositories]);

  const handleGitRepoInfoUpdated = useCallback((repo: AuthorizedGitRepositoryView, action: string) => {
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
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('GitRepositoryInfoUpdated', handleGitRepoInfoUpdated);
    },
    [handleGitRepoInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('GitRepositoryInfoUpdated', handleGitRepoInfoUpdated);
    },
    [handleGitRepoInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'git-repositories',
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, gitRepos, capabilities, isLoading, selectedTagNames };
};
