import { useEffect, useState, useCallback } from 'react';
import { PlatformsView, PlatformView } from '@/api/_generated';
import { HubConnection } from '@microsoft/signalr';
import { useContextSelector } from 'use-context-selector';
import { AuthContext } from '@/features/auth/AuthProvider';
import { configureHub, IHubConfig, startConnectionWithRetry } from '@/lib/signalr.helpers';

export enum ConnectionState {
  unknown,
  connecting,
  connected,
}

const usePlatformHub = () => {
  const [isLoading, setIsLoading] = useState(false);
  const [connectionState, setConnectionState] = useState(ConnectionState.unknown);
  const [platformsMessage, setPlatformsMessage] = useState<PlatformView[] | undefined>();
  const accessToken = useContextSelector(AuthContext, (v) => v?.accessToken);
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

  const getPlatformsList = useCallback(async (hubConnection: HubConnection) => {
    const response = await hubConnection.invoke<PlatformsView>('GetPlatforms');
    if (response) {
      setPlatformsMessage(response.platforms);
    }
    return () => {};
  }, []);

  // Setup event listeners for the hub connection
  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.onreconnecting(() => setConnectionState(ConnectionState.connecting));
      hubConnection.onreconnected(() => {
        getPlatformsList(hubConnection);
        setConnectionState(ConnectionState.connected);
      });
      hubConnection.on('PlatformsUpdated', handlePlatformsUpdated);
      hubConnection.on('PlatformUpdated', handlePlatformUpdated);
    },
    [handlePlatformsUpdated, handlePlatformUpdated, getPlatformsList],
  );

  // Remove event listeners from the hub connection
  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('PlatformsUpdated');
    hubConnection.off('PlatformUpdated');
  }, []);

  useEffect(() => {
    if (!accessToken) return;

    let hubConnection: HubConnection;
    let isCanceled = false;

    const initHub = () => {
      const config: IHubConfig = {
        url: `${baseUrl}/hubs/platform`,
        accessToken,
      };
      hubConnection = configureHub(config);
    };
    const onConnected = () => {
      getPlatformsList(hubConnection);
      setConnectionState(ConnectionState.connected);
    };

    const connect = async () => {
      setupEventListeners(hubConnection);
      await startConnectionWithRetry(hubConnection, onConnected, isCanceled);
    };

    const cleanup = () => {
      isCanceled = true;
      if (hubConnection) {
        removeEventListeners(hubConnection);
        hubConnection.stop();
      }
    };

    initHub();
    connect();

    return cleanup;
  }, [accessToken, baseUrl, setupEventListeners, removeEventListeners, getPlatformsList]);

  return {
    connectionState,
    platformsMessage,
    isLoading,
  };
};

export default usePlatformHub;
