import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCallback, useMemo } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';

export const useGitReposGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const groupName = collectionGroup('git-repositories', selectedTagNames);
  const structuralSharing = useCollectionStructuralSharing(groupName);
  const { data, isLoading, error, refetch, isFetching } = useRead('listGitRepositories', readArgs, {
    enabled: filterReady,
    structuralSharing,
  });
  const capabilities = data?.data.capabilities;
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.gitRepositories : undefined;
  }, [refetch]);
  const [gitRepos, handleGitRepoInfoUpdated, onJoinedGroup, onCollectionSnapshot] = useCollectionRows(
    data?.data.gitRepositories,
    JSON.stringify(readArgs ?? {}),
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
    enabled: filterReady,
    groupName,
    onJoinedGroup,
    onCollectionSnapshot,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, gitRepos, capabilities, isLoading, selectedTagNames };
};
