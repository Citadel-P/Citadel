import { useCollectionRows } from '@/hooks/useCollectionRows';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCallback, useMemo } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { AuthorizedGitRepositoryView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';

export const useGitReposGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const groupName = 'git-repositories';
  const structuralSharing = useCollectionStructuralSharing(groupName);
  const { data, isLoading, error, refetch, isFetching } = useRead('listGitRepositories', readArgs, {
    structuralSharing,
  });
  const capabilities = data?.data.capabilities;
  const matches = useCallback(
    (repo: AuthorizedGitRepositoryView) =>
      selectedTagNames.every((selected) =>
        repo.tags.some((tag) => tag.name.toLowerCase() === selected.trim().toLowerCase() || tag.id === selected.trim()),
      ),
    [selectedTagNames],
  );
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.gitRepositories : undefined;
  }, [refetch]);
  const [gitRepos, handleGitRepoInfoUpdated, onJoinedGroup] = useCollectionRows(
    data?.data.gitRepositories,
    JSON.stringify(readArgs ?? {}),
    matches,
    reload,
  );

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
    groupName,
    onJoinedGroup,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, gitRepos, capabilities, isLoading, selectedTagNames };
};
