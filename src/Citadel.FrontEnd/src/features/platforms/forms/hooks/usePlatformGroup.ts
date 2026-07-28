import { useRead } from '@/lib/hooks';
import { HubConnection } from '@microsoft/signalr';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { normalizePlatform } from '../../hooks/usePlatformsGroup';
import { PlatformDescriptorDockerPlatformDescriptor, PlatformView } from '@/api/generated/api.types';
import { PlatformStatsBatchView } from '@/api/types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const usePlatformGroup = (id: string) => {
  const args = useMemo(() => ({ id }), [id]);
  const { data, isLoading } = useRead('getPlatfom', args, { enabled: Boolean(id) });
  const [platform, setPlatform] = useState<PlatformView | undefined>();
  const lastDataRef = useRef<PlatformView | undefined>(undefined);

  useEffect(() => {
    if (data?.data && data.data !== lastDataRef.current) {
      lastDataRef.current = data.data;
      setPlatform(normalizePlatform(data.data));
    }
  }, [data?.data]);

  const handlePlatformUpdated = useCallback(
    (updated: PlatformView) => {
      const normalized = normalizePlatform(updated);
      if (normalized.id !== id) return;

      setPlatform(normalized);
    },
    [id],
  );

  const handlePlatformStatsUpdated = useCallback(
    (stats: PlatformStatsBatchView) => {
      if (stats.platformId !== id) return;

      setPlatform((current) => {
        if (!current) return current;

        const next = { ...current };
        next.stats = [stats.stat];
        next.networkCount = stats.networkCount;
        next.volumeCount = stats.volumeCount;
        next.imageCount = stats.imageCount;
        next.memTotal = stats.memTotal;

        if (next.type === 'Docker') {
          const descriptor = {
            ...(next.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor),
          };

          descriptor.containerCount = stats.containerCount;
          descriptor.containersRunning = stats.containersRunning;
          descriptor.containersPaused = stats.containersPaused;
          descriptor.containersStopped = stats.containersStopped;
          descriptor.imageUsedBytes = stats.imageUsedBytes;
          descriptor.volumeUsedBytes = stats.volumeUsedBytes;

          next.platformDescriptor = descriptor;
        }

        return next;
      });
    },
    [id],
  );

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('PlatformUpdated', handlePlatformUpdated);
      hubConnection.on('PlatformStatsUpdated', handlePlatformStatsUpdated);
    },
    [handlePlatformUpdated, handlePlatformStatsUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('PlatformUpdated', handlePlatformUpdated);
      hubConnection.off('PlatformStatsUpdated', handlePlatformStatsUpdated);
    },
    [handlePlatformUpdated, handlePlatformStatsUpdated],
  );

  useSignalRGroup({
    groupName: 'platforms',
    skip: !id,
    setupEventListeners,
    removeEventListeners,
  });

  return { platform, isLoading };
};
