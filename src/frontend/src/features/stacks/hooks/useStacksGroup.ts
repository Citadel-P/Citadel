import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { useCallback, useMemo } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
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
  const { data, isLoading, error, refetch, isFetching } = useRead('listStacks', readArgs, { enabled: filterReady });
  const capabilities = data?.data.capabilities;

  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.stacks : undefined;
  }, [refetch]);
  const [stacks, handleStackCatalogUpdated, onJoinedGroup, onCollectionSnapshot] = useCollectionRows(
    data?.data.stacks,
    JSON.stringify(readArgs ?? {}),
    reload,
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('StackCatalogUpdated', handleStackCatalogUpdated);
    },
    [handleStackCatalogUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('StackCatalogUpdated', handleStackCatalogUpdated);
    },
    [handleStackCatalogUpdated],
  );

  useRealtimeGroup({
    enabled: filterReady,
    groupName: collectionGroup('stacks', selectedTagNames, selectedPlatformId || undefined),
    onJoinedGroup,
    onCollectionSnapshot,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, stacks, isLoading, capabilities, selectedTagNames, selectedPlatformId };
};
