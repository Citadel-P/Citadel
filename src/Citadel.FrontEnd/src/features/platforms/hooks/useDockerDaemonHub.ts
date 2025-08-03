import { useEffect, useState, useCallback, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { configureHub, IHubConfig, startConnectionWithRetry } from '@/lib/signalr.helpers';
import { useAuthContext } from '@/features/auth/AuthContext';
import { ContainerView } from '@/api/_generated';

export const useDockerDaemonHub = (platformId?: string) => {
  const [containerEvent, setContainerEvent] = useState<ContainerEvent | undefined>();
  const { accessToken } = useAuthContext();
  const groupName = `docker-daemon-${platformId}`;
  const baseUrl = import.meta.env.VITE_API_BASE_URL;
  const isCanceledRef = useRef(false);

  const handleContainerEventReceived = useCallback((container: ContainerView, eventType: string) => {
    setContainerEvent({ container, eventType });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.onreconnecting(() => console.log('Reconnecting...'));
      hubConnection.onreconnected(() => {
        console.log('Reconnected');
      });

      hubConnection.on('ContainerEventReceived', handleContainerEventReceived);
    },
    [handleContainerEventReceived],
  );

  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('ContainerEventReceived');
  }, []);

  useEffect(() => {
    if (!platformId || !accessToken) return;

    let hubConnection: HubConnection;

    const initHub = () => {
      const config: IHubConfig = {
        url: `${baseUrl}/hubs/docker-daemon`,
        accessToken,
      };
      hubConnection = configureHub(config);
    };

    const onConnected = () => {
      hubConnection.send('JoinGroup', groupName).catch((error) => console.error('Failed to join group:', error));
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
  }, [accessToken, baseUrl, groupName, platformId, setupEventListeners, removeEventListeners]);

  return { containerEvent };
};

export interface ContainerEvent {
  container: ContainerView;
  eventType: string;
}
