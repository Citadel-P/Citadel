import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useMemo } from 'react';
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
  const query = useRead('listGitRepositories', readArgs, {
    enabled: filterReady,
    structuralSharing,
  });
  const { error, refetch, isFetching, isLoading } = query;
  const capabilities = query.data?.data.capabilities;
  const gitRepos = useCollectionSubscription({
    groupName,
    eventName: 'GitRepositoryInfoUpdated',
    enabled: filterReady,
    field: 'gitRepositories',
    query,
  });
  return { error, refetch, isFetching, gitRepos, capabilities, isLoading, selectedTagNames };
};
