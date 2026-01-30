import { useState, useCallback } from 'react';
import { PlatformDescriptorDockerPlatformDescriptor, PlatformsView, PlatformView } from '@/api/generated/api.types';
import { HubConnection } from '@microsoft/signalr';
import { PlatformStatsBatchView } from '@/api/types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const usePlatformsGroup = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [platformsMessage, setPlatformsMessage] = useState<PlatformView[] | undefined>();

  const handlePlatformsUpdated = useCallback((platforms: PlatformView[]) => {
    const processed = platforms.map((s) => {
      if (Array.isArray(s.platformDescriptor)) {
        const desc = (s.platformDescriptor as any)[1];
        desc.$type = (s.platformDescriptor as any)[0];
        return { ...s, platformDescriptor: desc };
      }
      return s;
    });
    setPlatformsMessage(processed);
  }, []);

  const handlePlatformUpdated = useCallback((platform: PlatformView) => {
    setPlatformsMessage((currentPlatforms) => {
      if (!currentPlatforms) return [platform];

      const existingIndex = currentPlatforms.findIndex((p) => p.id === platform.id);

      if (existingIndex === -1) {
        return [...currentPlatforms, platform];
      } else {
        const updated = [...currentPlatforms];
        updated[existingIndex] = platform;
        return updated;
      }
    });
  }, []);

  const handlePlatformDeleted = useCallback((id: string) => {
    setPlatformsMessage((currentPlatforms) => {
      if (!currentPlatforms) return [];
      return currentPlatforms.filter((p) => p.id !== id);
    });
  }, []);

  const handlePlatformStatsUpdated = useCallback((stats: PlatformStatsBatchView) => {
    setPlatformsMessage((currentPlatforms) => {
      if (!currentPlatforms) return;

      const existingIndex = currentPlatforms.findIndex((p) => p.id === stats.platformId);
      if (existingIndex === -1) return currentPlatforms;

      const updatedPlatforms = [...currentPlatforms];
      const target = { ...updatedPlatforms[existingIndex] };

      target.stats = [stats.stat];
      target.networkCount = stats.networkCount;
      target.volumeCount = stats.volumeCount;
      target.imageCount = stats.imageCount;
      target.memTotal = stats.memTotal;

      if (target.type === 'Docker') {
        // Deep clone the descriptor to ensure React sees the change
        const descriptor = { ...(target.platformDescriptor as PlatformDescriptorDockerPlatformDescriptor) };
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

  const getPlatformsList = useCallback(async (hubConnection: HubConnection) => {
    try {
      setIsLoading(true);
      const response = await hubConnection.invoke<PlatformsView>('GetPlatforms');
      if (response && response.platforms) {
        const processed = response.platforms.map((s) => {
          // Check if it's already processed to avoid double-parsing on re-renders
          if (Array.isArray(s.platformDescriptor)) {
            const desc = (s.platformDescriptor as any)[1];
            desc.$type = (s.platformDescriptor as any)[0];
            return { ...s, platformDescriptor: desc };
          }
          return s;
        });
        setPlatformsMessage(processed);
      }
    } catch (err) {
      console.error('Failed to fetch platforms', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

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

  const onJoinedGroup = useCallback(
    (hubConnection: HubConnection) => {
      if (!hubConnection) return;
      getPlatformsList(hubConnection);
      hubConnection.onreconnected(() => {
        getPlatformsList(hubConnection);
      });
    },
    [getPlatformsList],
  );

  useSignalRGroup({
    groupName: 'platforms',
    setupEventListeners,
    removeEventListeners,
    onJoinedGroup,
  });

  return {
    platformsMessage,
    isLoading,
  };
};
