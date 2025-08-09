import { useState, useCallback } from 'react';
import { PlatformDescriptorDockerPlatformDescriptor, PlatformsView, PlatformView } from '@/api/_generated';
import { HubConnection } from '@microsoft/signalr';
import { useAuthContext } from '@/features/auth/AuthContext';
import { PlatformStatsBatchView } from '@/api/models';
import { useSignalRHub } from '@/hooks/useSignalRHub';

export const usePlatformsHub = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [platformsMessage, setPlatformsMessage] = useState<PlatformView[] | undefined>();
  const { accessToken } = useAuthContext();
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

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
        if (updatedPlatforms[existingIndex].platformDescriptor.$type === 'Docker') {
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
    setIsLoading(true);
    const response = await hubConnection.invoke<PlatformsView>('GetPlatforms');
    if (response) {
      setPlatformsMessage(response.platforms);
    }
    setIsLoading(false);
  }, []);

  // Setup event listeners for the hub connection
  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.onreconnecting(() => console.log('Reconnecting...'));
      hubConnection.onreconnected(() => {
        getPlatformsList(hubConnection);
      });
      hubConnection.on('PlatformsUpdated', handlePlatformsUpdated);
      hubConnection.on('PlatformUpdated', handlePlatformUpdated);
      hubConnection.on('PlatformsDeleted', handlePlatformDeleted);
      hubConnection.on('PlatformStatsUpdated', handlePlatformStatsUpdated);
    },
    [
      handlePlatformsUpdated,
      handlePlatformUpdated,
      handlePlatformDeleted,
      handlePlatformStatsUpdated,
      getPlatformsList,
    ],
  );

  // Remove event listeners from the hub connection
  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('PlatformsUpdated');
    hubConnection.off('PlatformUpdated');
    hubConnection.off('PlatformsDeleted');
    hubConnection.off('PlatformStatsUpdated');
  }, []);

  const onConnected = useCallback(
    (hubConnection: HubConnection) => {
      getPlatformsList(hubConnection);
    },
    [getPlatformsList],
  );

  useSignalRHub({
    url: `${baseUrl}/hubs/docker`,
    groupName: 'platforms',
    accessToken,
    setupEventListeners,
    removeEventListeners,
    onConnected,
  });

  return {
    platformsMessage,
    isLoading,
  };
};
