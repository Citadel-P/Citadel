import { collectionGroup } from '@/lib/collection-scope';
import { mergePlatformIoSample } from '../platform-io';
import { useState, useMemo, useCallback } from 'react';
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
  const { data, isLoading, error, refetch, isFetching } = useRead('listPlatforms', readArgs, { enabled: filterReady });
  const [platforms, setPlatforms] = useState<PlatformView[] | undefined>();
  const groupName = collectionGroup('platforms', selectedTagNames);
  const [liveScope, setLiveScope] = useState<string>();
  const onCollectionSnapshot = useCallback(
    (active: boolean) => {
      setLiveScope((current) => (active ? groupName : current === groupName ? undefined : current));
      if (active) setPlatforms([]);
    },
    [groupName],
  );
  const capabilities = data?.data.capabilities;
  const snapshot = data?.data.platforms;
  const [baseline, setBaseline] = useState<{ group: string; rows?: PlatformView[] }>();
  if (baseline?.group !== groupName || baseline?.rows !== snapshot) {
    setBaseline({ group: groupName, rows: snapshot });
    if (liveScope !== groupName) setPlatforms(snapshot?.map(normalizePlatform));
  }

  const handlePlatformsUpdated = useCallback((platforms: PlatformView[]) => {
    setPlatforms(platforms.map(normalizePlatform));
  }, []);

  const handlePlatformUpdated = useCallback((platform: PlatformView) => {
    const normalized = normalizePlatform(platform);

    setPlatforms((current) => {
      if (!current) return current;

      const existingIndex = current.findIndex((p) => p.id === normalized.id);

      if (existingIndex === -1) {
        return [...current, normalized];
      }

      const updated = [...current];
      updated[existingIndex] = normalized;

      return updated;
    });
  }, []);

  const handlePlatformDeleted = useCallback((id: string) => {
    setPlatforms((current) => current?.filter((p) => p.id !== id));
  }, []);

  const handlePlatformStatsUpdated = useCallback((stats: PlatformStatsBatchView) => {
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
  }, []);

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
    onCollectionSnapshot,
    groupName,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, platformsMessage: platforms, capabilities, isLoading, selectedTagNames };
};
