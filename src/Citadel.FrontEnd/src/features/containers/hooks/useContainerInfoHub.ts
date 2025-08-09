import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useAuthContext } from '@/features/auth/AuthContext';
import { DockerContainerView } from '@/api/models';
import { useDockerDaemonHub } from '@/features/platforms/hooks/useDockerDaemonHub';
import { useSignalRHub } from '@/hooks/useSignalRHub';

export const useContainerInfoHub = (containerId?: string, platformId?: string) => {
  const { accessToken } = useAuthContext();
  const { containerEvent } = useDockerDaemonHub(platformId);

  const [containerInfo, setContainerInfo] = useState<DockerContainerView | undefined>();
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

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

  useSignalRHub({
    url: `${baseUrl}/hubs/docker`,
    accessToken,
    groupName: `container-info:${containerId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !containerId,
  });

  useEffect(() => {
    if (
      containerId &&
      containerEvent?.container.containerId.startsWith(containerId) &&
      containerEvent?.eventType !== 'destroy'
    ) {
      const container: DockerContainerView = {
        containerId: containerEvent.container.containerId,
        name: containerEvent.container.name,
        state: containerEvent.container.state,
        created: containerEvent.container.created,
        image: containerEvent.container.image,
        stack: containerEvent.container.stack,
        containerStat: containerEvent.container.lastStats,
        containerPort: containerEvent.container.ports as any,
      };
      setContainerInfo(() => container);
    }
  }, [containerEvent, containerId]);

  return { containerInfo };
};
