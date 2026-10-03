import { useEffect, useState, useMemo, useCallback, useRef } from 'react';
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

const normalizeTagName = (name: string) => name.trim().toLowerCase();

type UsePlatformsGroupOptions = {
  useTagFilter?: boolean;
};

export const usePlatformsGroup = ({ useTagFilter = true }: UsePlatformsGroupOptions = {}) => {
  const { selectedTagNames: activeTagNames } = useResourceTagFilter();
  const selectedTagNames = useMemo(() => (useTagFilter ? activeTagNames : []), [activeTagNames, useTagFilter]);
  const readArgs = useMemo(() => {
    if (selectedTagNames.length === 0) return undefined;
    return { query: { tags: selectedTagNames } };
  }, [selectedTagNames]);
  const { data, isLoading, error, refetch, isFetching } = useRead('listPlatforms', readArgs);
  const [platforms, setPlatforms] = useState<PlatformView[] | undefined>();
  const lastFetchedRef = useRef<PlatformView[]>([]);
  const capabilities = data?.data?.capabilities;

  useEffect(() => {
    if (!data?.data) return;
    const newBase = data.data.platforms;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setPlatforms(newBase.map(normalizePlatform));
    }
  }, [data]);

  const matchesActiveFilters = useCallback(
    (platform: PlatformView) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((platform.tags ?? []).map((tag) => normalizeTagName(tag.name)));
      return selectedTagNames.every((tagName) => tagNames.has(normalizeTagName(tagName)));
    },
    [selectedTagNames],
  );

  const handlePlatformsUpdated = useCallback(
    (platforms: PlatformView[]) => {
      setPlatforms(platforms.map(normalizePlatform).filter(matchesActiveFilters));
    },
    [matchesActiveFilters],
  );

  const handlePlatformUpdated = useCallback(
    (platform: PlatformView) => {
      const normalized = normalizePlatform(platform);

      setPlatforms((current) => {
        if (!current) return current;

        const existingIndex = current.findIndex((p) => p.id === normalized.id);

        if (!matchesActiveFilters(normalized)) {
          return existingIndex === -1 ? current : current.filter((p) => p.id !== normalized.id);
        }

        if (existingIndex === -1) {
          return [...current, normalized];
        }

        const updated = [...current];
        updated[existingIndex] = normalized;

        return updated;
      });
    },
    [matchesActiveFilters],
  );

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

      target.stats = [stats.stat];
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

  const platformsMessage = useMemo(() => platforms?.filter(matchesActiveFilters), [platforms, matchesActiveFilters]);

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
    groupName: 'platforms',
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, platformsMessage, capabilities, isLoading, selectedTagNames };
};
