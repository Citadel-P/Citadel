import { useEffect, useState, useCallback, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { configureHub, IHubConfig, startConnectionWithRetry } from '@/lib/signalr.helpers';
import { useAuthContext } from '@/features/auth/AuthContext';
import { DockerContainerView } from '@/api/models';

const useContainersHub = (containerId?: string) => {
  const [containerInfo, setContainerInfo] = useState<DockerContainerView | undefined>();
  const { accessToken } = useAuthContext();
  const groupName = `container-${containerId}`;
  const baseUrl = import.meta.env.VITE_API_BASE_URL;
  const isCanceledRef = useRef(false);

  const handleContainerInfoUpdated = useCallback((container: DockerContainerView) => {
    setContainerInfo(container);
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.onreconnecting(() => console.log('Reconnecting...'));
      hubConnection.onreconnected(() => {
        console.log('Reconnected');
      });

      hubConnection.on('ReceiveContainerInfo', handleContainerInfoUpdated);
    },
    [handleContainerInfoUpdated],
  );

  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('ReceiveContainerInfo');
  }, []);

  useEffect(() => {
    if (!containerId || !accessToken) return;

    let hubConnection: HubConnection;

    const initHub = () => {
      const config: IHubConfig = {
        url: `${baseUrl}/hubs/container-info`,
        accessToken,
      };
      hubConnection = configureHub(config);
    };

    const onConnected = () => {
      hubConnection.send('Subscribe', containerId).catch((error) => console.error('Failed to join group:', error));
    };

    const connect = async () => {
      setupEventListeners(hubConnection);
      await startConnectionWithRetry(hubConnection, onConnected, isCanceledRef);
    };

    const cleanup = () => {
      isCanceledRef.current = true;
      if (hubConnection) {
        hubConnection.send('Unsubscribe', containerId);
        removeEventListeners(hubConnection);
        hubConnection.stop();
      }
    };

    initHub();
    connect();

    return cleanup;
  }, [accessToken, baseUrl, groupName, containerId, setupEventListeners, removeEventListeners]);

  return { containerInfo };
};

export default useContainersHub;
