import { useState, useMemo, useCallback } from 'react';
import { PlatformDescriptorDockerPlatformDescriptor, PlatformView } from '@/api/generated/api.types';
import { HubConnection } from '@microsoft/signalr';
import { PlatformStatsBatchView } from '@/api/types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';

const normalizePlatform = (s: PlatformView): PlatformView => {
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

export const usePlatformsGroup = () => {
  const { selectedTagIds } = useResourceTagFilter();
  const { data, isLoading } = useRead(
    'listPlatforms',
    selectedTagIds.length > 0 ? { query: { tagIds: selectedTagIds } } : undefined,
  );
  const [realtimePlatforms, setRealtimePlatforms] = useState<PlatformView[] | null>(null);
  const capabilities = data?.data.capabilities;

  const platformsMessage = useMemo(() => {
    if (realtimePlatforms) {
      return realtimePlatforms;
    }

    return data?.data.platforms.map(normalizePlatform) ?? [];
  }, [realtimePlatforms, data]);

  const handlePlatformsUpdated = useCallback((platforms: PlatformView[]) => {
    setRealtimePlatforms(platforms.map(normalizePlatform));
  }, []);

  const handlePlatformUpdated = useCallback(
    (platform: PlatformView) => {
      const normalized = normalizePlatform(platform);

      setRealtimePlatforms((current) => {
        const source = current ?? data?.data.platforms.map(normalizePlatform) ?? [];

        const existingIndex = source.findIndex((p) => p.id === normalized.id);

        if (existingIndex === -1) {
          return [...source, normalized];
        }

        const updated = [...source];
        updated[existingIndex] = normalized;

        return updated;
      });
    },
    [data],
  );

  const handlePlatformDeleted = useCallback(
    (id: string) => {
      setRealtimePlatforms((current) => {
        const source = current ?? data?.data.platforms.map(normalizePlatform) ?? [];

        return source.filter((p) => p.id !== id);
      });
    },
    [data],
  );

  const handlePlatformStatsUpdated = useCallback(
    (stats: PlatformStatsBatchView) => {
      setRealtimePlatforms((current) => {
        const source = current ?? data?.data.platforms.map(normalizePlatform) ?? [];

        const existingIndex = source.findIndex((p) => p.id === stats.platformId);

        if (existingIndex === -1) {
          return source;
        }

        const updatedPlatforms = [...source];
        const target = { ...updatedPlatforms[existingIndex] };

        target.stats = [stats.stat];
        target.networkCount = stats.networkCount;
        target.volumeCount = stats.volumeCount;
        target.imageCount = stats.imageCount;
        target.memTotal = stats.memTotal;

        if (target.type === 'Docker') {
          const descriptor = {
            ...(target.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor),
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
    [data],
  );

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('PlatformsUpdated', handlePlatformsUpdated);
      hubConnection.on('PlatformUpdated', handlePlatformUpdated);
      hubConnection.on('PlatformsDeleted', handlePlatformDeleted);
      hubConnection.on('PlatformStatsUpdated', handlePlatformStatsUpdated);
    },
    [handlePlatformsUpdated, handlePlatformUpdated, handlePlatformDeleted, handlePlatformStatsUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('PlatformsUpdated', handlePlatformsUpdated);
      hubConnection.off('PlatformUpdated', handlePlatformUpdated);
      hubConnection.off('PlatformsDeleted', handlePlatformDeleted);
      hubConnection.off('PlatformStatsUpdated', handlePlatformStatsUpdated);
    },
    [handlePlatformsUpdated, handlePlatformUpdated, handlePlatformDeleted, handlePlatformStatsUpdated],
  );

  useSignalRGroup({
    groupName: 'platforms',
    setupEventListeners,
    removeEventListeners,
  });

  return {
    platformsMessage,
    capabilities,
    isLoading,
    selectedTagIds,
  };
};
