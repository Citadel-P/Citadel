import { useCollectionRows } from '@/hooks/useCollectionRows';
import { useCallback, useMemo } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { StackView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';
import { useResourcePlatformFilter } from '@/features/platforms/platform-filter';

export const useStacksGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const { selectedPlatformId } = useResourcePlatformFilter();
  const readArgs = useMemo(() => {
    const query: { catalog: true; tags?: string[]; platformId?: string } = { catalog: true };
    if (selectedTagNames.length > 0) query.tags = selectedTagNames;
    if (selectedPlatformId) query.platformId = selectedPlatformId;
    return { query };
  }, [selectedPlatformId, selectedTagNames]);
  const { data, isLoading, error, refetch, isFetching } = useRead('listStacks', readArgs);
  const capabilities = data?.data.capabilities;
  const matchesActiveFilters = useCallback(
    (stack: StackView) => {
      if (selectedPlatformId && stack.platformId !== selectedPlatformId) return false;
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((stack.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedPlatformId, selectedTagNames],
  );

  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.stacks : undefined;
  }, [refetch]);
  const [stacks, handleStackCatalogUpdated, onJoinedGroup] = useCollectionRows(
    data?.data.stacks,
    JSON.stringify(readArgs ?? {}),
    matchesActiveFilters,
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
    groupName: 'stacks',
    onJoinedGroup,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, stacks, isLoading, capabilities, selectedTagNames, selectedPlatformId };
};
