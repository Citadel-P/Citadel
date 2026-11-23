import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
import { useMemo } from 'react';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';
import { useResourcePlatformFilter } from '@/features/platforms/platform-filter';

export const useStacksGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const { selectedPlatformId } = useResourcePlatformFilter();
  const readArgs = useMemo(() => {
    const query: { catalog: true; tags?: string[]; platformId?: string } = { catalog: true };
    if (selectedTagNames.length > 0) query.tags = selectedTagNames;
    if (selectedPlatformId) query.platformId = selectedPlatformId;
    return { query };
  }, [selectedPlatformId, selectedTagNames]);
  const query = useRead('listStacks', readArgs, {
    enabled: filterReady,
  });
  const { error, refetch, isFetching, isLoading } = query;
  const capabilities = query.data?.data.capabilities;

  const stacks = useCollectionSubscription({
    groupName: collectionGroup('stacks', selectedTagNames, selectedPlatformId || undefined),
    eventName: 'StackCatalogUpdated',
    enabled: filterReady,
    field: 'stacks',
    query,
  });
  return { error, refetch, isFetching, stacks, isLoading, capabilities, selectedTagNames, selectedPlatformId };
};
