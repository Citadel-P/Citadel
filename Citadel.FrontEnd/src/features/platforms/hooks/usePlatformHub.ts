import { useEffect, useState, useCallback, useRef } from 'react';
import { PlatformDescriptorDockerPlatformDescriptor, PlatformsView, PlatformView } from '@/api/_generated';
import { HubConnection } from '@microsoft/signalr';
import { useContextSelector } from 'use-context-selector';
import { AuthContext } from '@/features/auth/AuthProvider';
import { configureHub, IHubConfig, startConnectionWithRetry } from '@/lib/signalr.helpers';
import { PlatformStatsBatchView } from '@/api/models';

const usePlatformHub = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [platformsMessage, setPlatformsMessage] = useState<PlatformView[] | undefined>();
  const accessToken = useContextSelector(AuthContext, (v) => v?.accessToken);
  const groupName = `Platforms`;
  const baseUrl = import.meta.env.VITE_API_BASE_URL;
  const isCanceledRef = useRef(false);

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
        updatedPlatforms[existingIndex].networkCount = platform.networksCount;
        updatedPlatforms[existingIndex].volumeCount = platform.volumesCount;

        updatedPlatforms[existingIndex].imageCount = platform.images;
        updatedPlatforms[existingIndex].memTotal = platform.memTotal;
        if (updatedPlatforms[existingIndex].platformDescriptor.$type === 'Docker') {
          const descriptor = updatedPlatforms[existingIndex]
            .platformDescriptor as PlatformDescriptorDockerPlatformDescriptor;
          descriptor.containerCount = platform.containers;
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

  useEffect(() => {
    if (!accessToken) return;

    let hubConnection: HubConnection;

    const initHub = () => {
      const config: IHubConfig = {
        url: `${baseUrl}/hubs/platform`,
        accessToken,
      };
      hubConnection = configureHub(config);
    };
    const onConnected = () => {
      hubConnection
        .send('JoinGroup', groupName)
        .then(() => getPlatformsList(hubConnection))
        .catch((error) => console.error('Failed to join group:', error));
    };

    const connect = async () => {
      setupEventListeners(hubConnection);
      await startConnectionWithRetry(hubConnection, onConnected, isCanceledRef);
    };

    const cleanup = () => {
      isCanceledRef.current = true;
      if (hubConnection) {
        hubConnection.send('LeaveGroup', groupName);
        removeEventListeners(hubConnection);
        hubConnection.stop();
      }
    };

    initHub();
    connect();

    return cleanup;
  }, [accessToken, baseUrl, groupName, setupEventListeners, removeEventListeners, getPlatformsList]);

  return {
    platformsMessage,
    isLoading,
  };
};

export default usePlatformHub;
