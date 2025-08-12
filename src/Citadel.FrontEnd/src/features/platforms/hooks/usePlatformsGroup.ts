import { useState, useCallback } from 'react';
import { PlatformDescriptorDockerPlatformDescriptor, PlatformsView, PlatformView } from '@/api/_generated';
import { HubConnection } from '@microsoft/signalr';
import { PlatformStatsBatchView } from '@/api/models';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const usePlatformsGroup = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [platformsMessage, setPlatformsMessage] = useState<PlatformView[] | undefined>();

  // Callback to handle platform updates
  const handlePlatformsUpdated = useCallback((platforms: PlatformView[]) => {
    setPlatformsMessage(platforms);
  }, []);

  const handlePlatformUpdated = useCallback((platform: PlatformView) => {
    setPlatformsMessage((currentPlatforms) => {
      if (!currentPlatforms) return;

      const updatedPlatforms = [...(currentPlatforms ?? [])];

      const existingIndex = updatedPlatforms.findIndex((p) => p.id === platform.id);

      // Add platform if it doesn't exist
      if (existingIndex === -1) {
        return {
          ...currentPlatforms,
          platform,
        };
      } else {
        updatedPlatforms[existingIndex] = platform;
        return updatedPlatforms;
      }
    });
  }, []);

  const handlePlatformDeleted = useCallback((id: string) => {
    setPlatformsMessage((currentPlatforms) => {
      if (!currentPlatforms) return;

      const updatedPlatforms = currentPlatforms.filter((p) => p.id !== id);
      if (updatedPlatforms.length === currentPlatforms.length) {
        return currentPlatforms;
      }
      return updatedPlatforms;
    });
  }, []);

  const handlePlatformStatsUpdated = useCallback((platform: PlatformStatsBatchView) => {
    setPlatformsMessage((currentPlatforms) => {
      if (!currentPlatforms) return;
      const updatedPlatforms = [...(currentPlatforms ?? [])];
      const existingIndex = updatedPlatforms.findIndex((p) => p.id === platform.platformId);

      if (existingIndex !== -1) {
        updatedPlatforms[existingIndex].stats = [platform.stat];
        updatedPlatforms[existingIndex].networkCount = platform.networkCount;
        updatedPlatforms[existingIndex].volumeCount = platform.volumeCount;

        updatedPlatforms[existingIndex].imageCount = platform.imageCount;
        updatedPlatforms[existingIndex].memTotal = platform.memTotal;
        if (updatedPlatforms[existingIndex].type === 'Docker') {
          const descriptor = updatedPlatforms[existingIndex]
            .platformDescriptor as PlatformDescriptorDockerPlatformDescriptor;
          descriptor.containerCount = platform.containerCount;
          descriptor.containersRunning = platform.containersRunning;
          descriptor.containersPaused = platform.containersPaused;
          descriptor.containersStopped = platform.containersStopped;
        }
      }
      return updatedPlatforms;
    });
  }, []);

  const getPlatformsList = useCallback(async (hubConnection: HubConnection) => {
    try {
      setIsLoading(true);
      const response = await hubConnection.invoke<PlatformsView>('GetPlatforms');
      if (response) {
        response.platforms.map((s) => {
          s.platformDescriptor = (s.platformDescriptor as any)[1]; // message pack derived type
          s.platformDescriptor.$type = (s.platformDescriptor as any)[0];
        });
        setPlatformsMessage(response.platforms);
      }
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Setup event listeners for the hub connection
  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('PlatformsUpdated', handlePlatformsUpdated);
      hubConnection.on('PlatformUpdated', handlePlatformUpdated);
      hubConnection.on('PlatformsDeleted', handlePlatformDeleted);
      hubConnection.on('PlatformStatsUpdated', handlePlatformStatsUpdated);
    },
    [handlePlatformsUpdated, handlePlatformUpdated, handlePlatformDeleted, handlePlatformStatsUpdated],
  );

  // Remove event listeners from the hub connection
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
