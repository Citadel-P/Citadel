import { collectionGroup } from '@/lib/collection-scope';
import { mergePlatformIoSample } from '../platform-io';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { useMemo, useCallback } from 'react';
import { PlatformView } from '@/api/generated/api.types';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { PlatformStatsBatchView } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';

export const normalizePlatform = (s: PlatformView): PlatformView => {
  if (Array.isArray(s.platformDescriptor)) {
    const desc = (s.platformDescriptor as any)[1];
    desc.$type = (s.platformDescriptor as any)[0];

    return {
      ...s,
      platformDescriptor: desc,
    };
  }

  return s;
};

type UsePlatformsGroupOptions = {
  useTagFilter?: boolean;
};

export const usePlatformsGroup = ({ useTagFilter = true }: UsePlatformsGroupOptions = {}) => {
  const { selectedTagNames: activeTagNames, filterReady } = useResourceTagFilter(useTagFilter);
  const selectedTagNames = useMemo(() => (useTagFilter ? activeTagNames : []), [activeTagNames, useTagFilter]);
  const readArgs = useMemo(() => {
    if (selectedTagNames.length === 0) return undefined;
    return { query: { tags: selectedTagNames } };
  }, [selectedTagNames]);
  const { data, isSuccess, dataUpdatedAt, isLoading, error, refetch, isFetching } = useRead('listPlatforms', readArgs, {
    enabled: filterReady,
  });
  const groupName = collectionGroup('platforms', selectedTagNames);
  const capabilities = data?.data.capabilities;
  const snapshot = useMemo(() => data?.data.platforms?.map(normalizePlatform), [data?.data.platforms]);
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.platforms.map(normalizePlatform) : undefined;
  }, [refetch]);
  const {
    rows: platforms,
    onEvent: update,
    onJoinedGroup,
    onCollectionSnapshot,
    transformRows: setPlatforms,
  } = useCollectionRows(snapshot, groupName, reload, isSuccess ? dataUpdatedAt : undefined);
  const handlePlatformsUpdated = useCallback(
    (rows: PlatformView[]) => {
      setPlatforms(() => rows.map(normalizePlatform));
    },
    [setPlatforms],
  );

  const handlePlatformUpdated = useCallback(
    (platform: PlatformView) => {
      update(normalizePlatform(platform), 'create');
    },
    [update],
  );
  const handlePlatformDeleted = useCallback(
    (id: string) => {
      setPlatforms((rows) => rows?.filter((row) => row.id !== id));
    },
    [setPlatforms],
  );

  const handlePlatformStatsUpdated = useCallback(
    (stats: PlatformStatsBatchView) => {
      setPlatforms((current) => {
        if (!current) return current;
        const existingIndex = current.findIndex((p) => p.id === stats.platformId);

        if (existingIndex === -1) {
          return current;
        }

        const updatedPlatforms = [...current];
        const target = { ...updatedPlatforms[existingIndex] };

        const previous = target.stats?.[0];
        const merged = mergePlatformIoSample(previous, stats.stat, stats.ioOnly ? stats.ioNodeId : undefined);
        if (merged === previous) return current;
        target.stats = [merged];
        if (stats.ioOnly) {
          updatedPlatforms[existingIndex] = target;
          return updatedPlatforms;
        }
        target.networkCount = stats.networkCount;
        target.volumeCount = stats.volumeCount;
        target.imageCount = stats.imageCount;
        target.memTotal = stats.memTotal;

        if (target.platformDescriptor?.$type === 'Docker') {
          const descriptor = {
            ...target.platformDescriptor,
            $type: 'Docker' as const,
          };

          descriptor.containerCount = stats.containerCount;
          descriptor.containersRunning = stats.containersRunning;
          descriptor.containersPaused = stats.containersPaused;
          descriptor.containersStopped = stats.containersStopped;

          target.platformDescriptor = descriptor;
        }

        updatedPlatforms[existingIndex] = target;

        return updatedPlatforms;
      });
    },
    [setPlatforms],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('PlatformsUpdated', handlePlatformsUpdated);
      hubConnection.on('PlatformUpdated', handlePlatformUpdated);
      hubConnection.on('PlatformsDeleted', handlePlatformDeleted);
      hubConnection.on('PlatformStatsUpdated', handlePlatformStatsUpdated);
    },
    [handlePlatformsUpdated, handlePlatformUpdated, handlePlatformDeleted, handlePlatformStatsUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('PlatformsUpdated', handlePlatformsUpdated);
      hubConnection.off('PlatformUpdated', handlePlatformUpdated);
      hubConnection.off('PlatformsDeleted', handlePlatformDeleted);
      hubConnection.off('PlatformStatsUpdated', handlePlatformStatsUpdated);
    },
    [handlePlatformsUpdated, handlePlatformUpdated, handlePlatformDeleted, handlePlatformStatsUpdated],
  );

  useRealtimeGroup({
    enabled: filterReady,
    onJoinedGroup,
    onCollectionSnapshot,
    groupName,
    setupEventListeners,
    removeEventListeners,
  });

  return {
    groupName,
    error,
    refetch,
    isFetching,
    platformsMessage: platforms,
    capabilities,
    isLoading,
    selectedTagNames,
  };
};
