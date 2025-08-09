import { useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useAuthContext } from '@/features/auth/AuthContext';
import { ContainerView } from '@/api/_generated';
import { useSignalRHub } from '@/hooks/useSignalRHub';

export const useDockerDaemonHub = (platformId?: string) => {
  const [containerEvent, setContainerEvent] = useState<ContainerEvent | undefined>();
  const { accessToken } = useAuthContext();
  const groupName = `docker-daemon:${platformId}`;
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

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

  useSignalRHub({
    url: `${baseUrl}/hubs/docker`,
    groupName: groupName,
    accessToken,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });

  return { containerEvent };
};

export interface ContainerEvent {
  container: ContainerView;
  eventType: string;
}
