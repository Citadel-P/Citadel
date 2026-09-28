import { BuildAgentPoolView, ResourceCapabilities } from '@/api/generated/api.types';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

export const useBuildPoolsGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading, error, refetch, isFetching } = useRead('listBuildAgentPools', readArgs);
  const [pools, setPools] = useState<BuildAgentPoolView[] | undefined>();
  const [capabilities, setCapabilities] = useState<ResourceCapabilities | undefined>();
  const lastFetchedRef = useRef<BuildAgentPoolView[]>([]);

  useEffect(() => {
    if (!data) return;

    const next = data.data.pools;
    if (next !== lastFetchedRef.current) {
      lastFetchedRef.current = next;
      setPools(next);
      setCapabilities(data.data.capabilities);
    }
  }, [data]);

  const matchesActiveFilters = useCallback(
    (pool: BuildAgentPoolView) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((pool.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedTagNames],
  );

  const handleBuildAgentPoolInfoUpdated = useCallback(
    (pool: BuildAgentPoolView, action: string) => {
      setPools((prev) => {
        if (!prev) return prev;
        if (action === 'create') {
          return matchesActiveFilters(pool) ? [...prev, pool] : prev;
        }
        if (action === 'delete') {
          return prev.filter((item) => item.id !== pool.id);
        }

        const index = prev.findIndex((item) => item.id === pool.id);
        if (!matchesActiveFilters(pool)) {
          return index === -1 ? prev : prev.filter((item) => item.id !== pool.id);
        }

        if (index === -1) return [...prev, pool];

        const updated = [...prev];
        updated[index] = pool;
        return updated;
      });
    },
    [matchesActiveFilters],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('BuildAgentPoolInfoUpdated', handleBuildAgentPoolInfoUpdated);
    },
    [handleBuildAgentPoolInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('BuildAgentPoolInfoUpdated', handleBuildAgentPoolInfoUpdated);
    },
    [handleBuildAgentPoolInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'build-agent-pools',
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, pools: pools ?? [], isLoading, capabilities };
};
