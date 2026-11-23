import { useCollectionRows } from '@/hooks/useCollectionRows';
import { collectionGroup } from '@/lib/collection-scope';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useMemo } from 'react';

export const useBuildPoolsGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading, error, refetch, isFetching } = useRead('listBuildAgentPools', readArgs, {
    enabled: filterReady,
  });
  const groupName = collectionGroup('build-agent-pools', selectedTagNames);
  const capabilities = data?.data.capabilities;
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.pools : undefined;
  }, [refetch]);
  const [pools, handleBuildAgentPoolInfoUpdated, onJoinedGroup, onCollectionSnapshot] = useCollectionRows(
    data?.data.pools,
    groupName,
    reload,
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
    enabled: filterReady,
    groupName,
    onJoinedGroup,
    onCollectionSnapshot,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, pools: pools ?? [], isLoading, capabilities };
};
