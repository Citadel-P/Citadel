import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
import { collectionGroup } from '@/lib/collection-scope';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';

export const useBuildPoolsGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const query = useRead('listBuildAgentPools', readArgs, {
    enabled: filterReady,
  });
  const groupName = collectionGroup('build-agent-pools', selectedTagNames);
  const { error, refetch, isFetching, isLoading } = query;
  const capabilities = query.data?.data.capabilities;
  const pools = useCollectionSubscription({
    groupName,
    eventName: 'BuildAgentPoolInfoUpdated',
    enabled: filterReady,
    field: 'pools',
    query,
  });
  return { error, refetch, isFetching, pools: pools ?? [], isLoading, capabilities };
};
