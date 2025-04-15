import { useEffect, useState, useCallback } from 'react';
import { PlatformView } from '@/api/_generated';
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
  const [connectionState, setConnectionState] = useState(ConnectionState.unknown);
  const [platformsMessage, setPlatformsMessage] = useState<PlatformView[] | undefined>();
  const accessToken = useContextSelector(AuthContext, (v) => v?.accessToken);
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

  // Callback to handle platform updates
  const handlePlatformsUpdated = useCallback((platforms: PlatformView[]) => {
    setPlatformsMessage(platforms);
  }, []);

  // Setup event listeners for the hub connection
  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.onreconnecting(() => setConnectionState(ConnectionState.connecting));
      hubConnection.onreconnected(() => setConnectionState(ConnectionState.connected));
      hubConnection.on('PlatformsUpdated', handlePlatformsUpdated);
    },
    [handlePlatformsUpdated],
  );

  // Remove event listeners from the hub connection
  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('PlatformsUpdated');
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

    const connect = async () => {
      setupEventListeners(hubConnection);
      await startConnectionWithRetry(hubConnection, () => setConnectionState(ConnectionState.connected), isCanceled);
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
  }, [accessToken, baseUrl, setupEventListeners, removeEventListeners]);

  return {
    connectionState,
    platformsMessage,
  };
};

export default usePlatformHub;
