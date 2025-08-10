import { useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ContainerView } from '@/api/_generated';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useDockerDaemonGroup = (platformId?: string) => {
  const [containerEvent, setContainerEvent] = useState<ContainerEvent | undefined>();

  const handleContainerEventReceived = useCallback((container: ContainerView, eventType: string) => {
    setContainerEvent({ container, eventType });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ContainerEventReceived', handleContainerEventReceived);
    },
    [handleContainerEventReceived],
  );

  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('ContainerEventReceived', handleContainerEventReceived);
  }, [handleContainerEventReceived]);

  useSignalRGroup({
    groupName: `docker-daemon:${platformId}`,
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