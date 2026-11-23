import { useCollectionRows } from '@/hooks/useCollectionRows';
import { collectionGroup } from '@/lib/collection-scope';
import { useCallback, useMemo } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';
import { useResourcePlatformFilter } from '@/features/platforms/platform-filter';

export const useDeploymentsGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const { selectedPlatformId } = useResourcePlatformFilter();
  const readArgs = useMemo(() => {
    const query: { tags?: string[]; platformId?: string } = {};
    if (selectedTagNames.length > 0) query.tags = selectedTagNames;
    if (selectedPlatformId) query.platformId = selectedPlatformId;
    return Object.keys(query).length > 0 ? { query } : undefined;
  }, [selectedPlatformId, selectedTagNames]);
  const { data, isLoading, error, refetch, isFetching } = useRead('listDeployments', readArgs, {
    enabled: filterReady,
  });
  const groupName = collectionGroup('deployments', selectedTagNames, selectedPlatformId || undefined);
  const capabilities = data?.data.capabilities;
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.deployments : undefined;
  }, [refetch]);
  const [deployments, handleDeploymentInfoUpdated, onJoinedGroup, onCollectionSnapshot] = useCollectionRows(
    data?.data.deployments,
    groupName,
    reload,
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('DeploymentInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('DeploymentInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  useRealtimeGroup({
    enabled: filterReady,
    groupName,
    onJoinedGroup,
    onCollectionSnapshot,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, deployments, isLoading, capabilities, selectedTagNames, selectedPlatformId };
};
